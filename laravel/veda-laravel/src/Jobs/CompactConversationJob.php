<?php

namespace Veda\Laravel\Jobs;

use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;
use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use Veda\Laravel\Services\ConversationCompactionStore;
use Veda\Laravel\Services\ConversationSummarizationService;
use Veda\Laravel\Services\ConversationSummarizer;
use Veda\Laravel\Services\ConversationTranscriptWriter;

class CompactConversationJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $timeout = 120;

    public int $tries = 2;

    /**
     * @param  array<int, array<string, mixed>>  $toSummarize
     */
    public function __construct(
        public int|string $userId,
        public string $chatId,
        public array $toSummarize = [],
    ) {}

    public function handle(
        ConversationSummarizationService $summarizationService,
        ConversationCompactionStore $store,
        ConversationTranscriptWriter $transcriptWriter,
    ): void {
        if (! (bool) config('veda.compaction.enabled', true)) {
            return;
        }

        $lockKey = "veda_compaction_lock:{$this->userId}:{$this->chatId}";
        $lock = Cache::lock($lockKey, (int) config('veda.compaction.lock_seconds', 180));
        if (! $lock->get()) {
            return;
        }

        try {
            $userModel = config('veda.user_model');
            $user = is_string($userModel) && class_exists($userModel)
                ? $userModel::query()->find($this->userId)
                : null;

            if (! $user) {
                return;
            }

            $toSummarize = $this->toSummarize;
            if ($toSummarize === []) {
                $messages = DB::table(config('veda.tables.messages') ?? config('ai.conversations.tables.messages', 'agent_conversation_messages'))
                    ->where('conversation_id', $this->chatId)
                    ->orderBy('created_at', 'asc')
                    ->get();

                $minMessages = (int) config('veda.compaction.min_messages', 40);
                $keepTail = (int) config('veda.compaction.keep_tail_messages', 20);

                if ($messages->count() <= $minMessages) {
                    return;
                }

                $messagesToSummarize = $messages->slice(0, $messages->count() - $keepTail);

                foreach ($messagesToSummarize as $msg) {
                    $toSummarize[] = [
                        'role' => $msg->role,
                        'content' => $msg->content,
                        'tool_calls' => json_decode($msg->tool_calls ?? '[]', true),
                    ];
                }
            }

            if ($toSummarize === []) {
                return;
            }

            $transcriptPath = $transcriptWriter->append($user, $this->chatId, $toSummarize);

            $summary = $summarizationService->summarize($toSummarize);
            if ($summary === '') {
                $summary = (new ConversationSummarizer)->buildExtractiveSummary($toSummarize);
            }

            if ($summary === '') {
                return;
            }

            $store->saveFrozenSummary(
                $user,
                $this->chatId,
                $summary,
                $transcriptPath,
                $store->fingerprint($toSummarize),
            );

            Log::info('veda.conversation_compaction.done', [
                'user_id' => $this->userId,
                'chat_id' => $this->chatId,
            ]);
        } finally {
            optional($lock)->release();
            Cache::forget("veda_compaction_pending:{$this->userId}:{$this->chatId}");
        }
    }
}
