<?php

namespace Veda\Laravel\Jobs;

use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\SerializesModels;
use Illuminate\Support\Facades\DB;
use Veda\Laravel\Models\VedaChatHistory;
use Veda\Laravel\Services\ChatTitleService;
use Veda\Laravel\Services\ConversationMessageExporter;

class GenerateChatTitleJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $timeout = 60;

    public int $tries = 2;

    public function __construct(
        public int|string $userId,
        public string $chatId,
        public string $userRequest,
    ) {}

    public function handle(
        ChatTitleService $titleService,
        ConversationMessageExporter $exporter,
    ): void {
        $history = VedaChatHistory::query()
            ->where('user_id', $this->userId)
            ->where('chat_id', $this->chatId)
            ->first();

        if (! $titleService->shouldAutoGenerate($history?->title)) {
            return;
        }

        $userRequest = trim($this->userRequest);
        if ($userRequest === '') {
            $exported = $exporter->export($this->chatId);
            $userRequest = $titleService->extractFirstUserRequest($exported['messages']);
        }

        if ($userRequest === '') {
            return;
        }

        $generatedTitle = $titleService->generateTitle($userRequest);
        if ($generatedTitle === '') {
            $generatedTitle = $titleService->deriveProvisionalTitle($userRequest);
        }

        if ($generatedTitle === '') {
            return;
        }

        VedaChatHistory::query()
            ->where('user_id', $this->userId)
            ->where('chat_id', $this->chatId)
            ->update(['title' => $generatedTitle]);

        DB::table(config('veda.tables.conversations') ?? config('ai.conversations.tables.conversations', 'agent_conversations'))
            ->where('id', $this->chatId)
            ->update(['title' => $generatedTitle]);
    }
}
