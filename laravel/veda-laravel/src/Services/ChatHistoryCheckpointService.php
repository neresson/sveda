<?php

namespace Veda\Laravel\Services;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Support\Facades\DB;
use Veda\Laravel\Models\VedaChatHistory;

class ChatHistoryCheckpointService
{
    public function __construct(
        protected ConversationMessageExporter $exporter,
        protected ChatHistorySyncService $syncService,
    ) {}

    /**
     * @param  array{title?: string|null, tokensUsed?: int|null}  $context
     * @return array{version: int, updatedAt: int}|null
     */
    public function checkpointAfterAgentStream(
        Authenticatable $user,
        string $chatId,
        array $context = [],
        ?EmbedChatScope $scope = null,
    ): ?array {
        $exported = $this->exporter->export($chatId);
        if ($exported['messages'] === []) {
            return null;
        }

        $scope ??= new EmbedChatScope($user, false);

        return DB::transaction(function () use ($scope, $chatId, $exported, $context) {
            $existing = $scope->applyToHistoryQuery(VedaChatHistory::query())
                ->where('chat_id', $chatId)
                ->first();

            $messages = $this->syncService->resolveMessagesForSync(
                $existing?->messages ?? [],
                $exported['messages']
            );
            $conversationHistory = $this->syncService->resolveConversationHistoryForSync(
                $existing?->conversation_history ?? [],
                $exported['conversationHistory']
            );

            $title = $context['title'] ?? null;
            if (! is_string($title) || trim($title) === '') {
                $title = $existing?->title;
            }

            $nextVersion = ($existing?->version ?? 0) + 1;

            $history = VedaChatHistory::query()->updateOrCreate(
                $scope->uniqueKeysForChat($chatId),
                array_merge($scope->attributesForCreate($chatId), [
                    'title' => $title,
                    'messages' => $messages,
                    'conversation_history' => $conversationHistory,
                    'tokens_used' => max(
                        (int) ($existing?->tokens_used ?? 0),
                        max(0, (int) ($context['tokensUsed'] ?? 0))
                    ),
                    'version' => $nextVersion,
                    'updated_at' => now(),
                    'created_at' => $existing?->created_at ?? now(),
                ])
            );

            return [
                'version' => (int) $history->version,
                'updatedAt' => $history->updated_at?->getTimestampMs() ?? now()->getTimestampMs(),
            ];
        });
    }
}
