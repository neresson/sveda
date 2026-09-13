<?php

namespace Veda\Laravel\Services;

use Illuminate\Contracts\Auth\Authenticatable;
use Veda\Laravel\Models\VedaChatCompaction;

class ConversationCompactionStore
{
    public function isEnabled(): bool
    {
        return (bool) config('veda.compaction.enabled', true);
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     */
    public function fingerprint(array $messages): string
    {
        $json = json_encode($messages, JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE);

        return hash('sha256', is_string($json) ? $json : '');
    }

    public function findLatestSummary(Authenticatable $user, string $chatId): ?VedaChatCompaction
    {
        if (! $this->isEnabled()) {
            return null;
        }

        $row = VedaChatCompaction::query()
            ->where('user_id', $user->getAuthIdentifier())
            ->where('chat_id', $chatId)
            ->orderByDesc('summarized_at')
            ->first();

        if (! $row instanceof VedaChatCompaction) {
            return null;
        }

        return trim($row->summary_text) !== '' ? $row : null;
    }

    public function findFrozenSummary(Authenticatable $user, string $chatId, string $fingerprint): ?VedaChatCompaction
    {
        if (! $this->isEnabled()) {
            return null;
        }

        $row = VedaChatCompaction::query()
            ->where('user_id', $user->getAuthIdentifier())
            ->where('chat_id', $chatId)
            ->first();

        if (! $row instanceof VedaChatCompaction) {
            return null;
        }

        if ($row->source_fingerprint !== $fingerprint) {
            return null;
        }

        return trim($row->summary_text) !== '' ? $row : null;
    }

    public function saveFrozenSummary(
        Authenticatable $user,
        string $chatId,
        string $summaryText,
        string $transcriptPath,
        string $fingerprint,
    ): VedaChatCompaction {
        return VedaChatCompaction::query()->updateOrCreate(
            [
                'user_id' => $user->getAuthIdentifier(),
                'chat_id' => $chatId,
            ],
            [
                'summary_text' => $summaryText,
                'transcript_path' => $transcriptPath,
                'source_fingerprint' => $fingerprint,
                'summarized_at' => now(),
            ]
        );
    }
}
