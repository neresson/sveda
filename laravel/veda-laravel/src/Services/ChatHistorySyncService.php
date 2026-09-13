<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\DB;
use Veda\Laravel\Models\VedaChatHistory;

class ChatHistorySyncService
{
    public function deleteOne(EmbedChatScope $scope, string $chatId): bool
    {
        return $scope->applyToHistoryQuery(VedaChatHistory::query())
            ->where('chat_id', $chatId)
            ->delete() > 0;
    }

    public function updateTitle(EmbedChatScope $scope, string $chatId, string $title): bool
    {
        $normalizedTitle = trim($title);
        if ($normalizedTitle === '') {
            return false;
        }

        $updated = $scope->applyToHistoryQuery(VedaChatHistory::query())
            ->where('chat_id', $chatId)
            ->update(['title' => $normalizedTitle]);

        if ($updated === 0) {
            return false;
        }

        if (! $scope->embedVisitorMode && $scope->userId !== null) {
            DB::table(config('veda.tables.conversations') ?? config('ai.conversations.tables.conversations', 'agent_conversations'))
                ->where('id', $chatId)
                ->where('user_id', $scope->userId)
                ->update(['title' => $normalizedTitle]);
        }

        return true;
    }

    /**
     * @param  array<int, array<string, mixed>>  $existing
     * @param  array<int, array<string, mixed>>  $incoming
     * @return array<int, array<string, mixed>>
     */
    public function resolveMessagesForSync(array $existing, array $incoming): array
    {
        if ($incoming === []) {
            return $incoming;
        }

        if ($existing === []) {
            return $incoming;
        }

        if ($this->messagesPayloadIsAtLeastAsComplete($existing, $incoming)) {
            return $incoming;
        }

        return $existing;
    }

    /**
     * @param  array<int, array<string, mixed>>  $existing
     * @param  array<int, array<string, mixed>>  $incoming
     * @return array<int, array<string, mixed>>
     */
    public function resolveConversationHistoryForSync(array $existing, array $incoming): array
    {
        return $this->resolveMessagesForSync($existing, $incoming);
    }

    /**
     * @param  array<int, mixed>  $existing
     * @param  array<int, mixed>  $incoming
     */
    public function messagesPayloadIsAtLeastAsComplete(array $existing, array $incoming): bool
    {
        $existingCount = count($existing);
        $incomingCount = count($incoming);

        if ($incomingCount > $existingCount) {
            return true;
        }

        if ($incomingCount < $existingCount) {
            return false;
        }

        return $this->estimateMessagesPayloadSize($incoming) >= $this->estimateMessagesPayloadSize($existing);
    }

    /**
     * @param  array<int, mixed>  $messages
     */
    public function estimateMessagesPayloadSize(array $messages): int
    {
        return strlen((string) json_encode($messages, JSON_UNESCAPED_UNICODE));
    }

    /**
     * @return array<string, mixed>
     */
    public function transformSummary(VedaChatHistory $history): array
    {
        return [
            'id' => $history->chat_id,
            'title' => $history->title,
            'preview' => $this->extractPreview($history->messages ?? []),
            'tokensUsed' => (int) ($history->tokens_used ?? 0),
            'version' => (int) ($history->version ?? 0),
            'createdAt' => $history->created_at?->getTimestampMs() ?? now()->getTimestampMs(),
            'updatedAt' => $history->updated_at?->getTimestampMs() ?? now()->getTimestampMs(),
        ];
    }

    /**
     * @return array<string, mixed>
     */
    public function transformHistory(VedaChatHistory $history): array
    {
        return [
            'id' => $history->chat_id,
            'title' => $history->title,
            'messages' => $history->messages ?? [],
            'conversationHistory' => $history->conversation_history ?? [],
            'tokensUsed' => (int) ($history->tokens_used ?? 0),
            'version' => (int) ($history->version ?? 0),
            'createdAt' => $history->created_at?->getTimestampMs() ?? now()->getTimestampMs(),
            'updatedAt' => $history->updated_at?->getTimestampMs() ?? now()->getTimestampMs(),
        ];
    }

    /**
     * @param  array<int, mixed>  $messages
     */
    protected function extractPreview(array $messages): string
    {
        foreach ($messages as $message) {
            if (! is_array($message) || ($message['role'] ?? '') !== 'user') {
                continue;
            }

            $content = $message['content'] ?? '';
            if (is_string($content) && trim($content) !== '') {
                return trim($content);
            }

            $parts = $message['parts'] ?? [];
            if (! is_array($parts)) {
                continue;
            }

            $text = collect($parts)
                ->filter(fn ($part) => is_array($part) && ($part['type'] ?? '') === 'text')
                ->map(fn ($part) => trim((string) ($part['text'] ?? '')))
                ->filter()
                ->join(' ');

            if ($text !== '') {
                return $text;
            }
        }

        return '';
    }
}
