<?php

namespace Veda\Laravel\Services;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Support\Facades\Storage;

class ConversationTranscriptWriter
{
    public function isEnabled(): bool
    {
        return (bool) config('veda.compaction.transcript_enabled', true);
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     */
    public function append(Authenticatable $user, string $chatId, array $messages): string
    {
        $relativePath = $this->relativePath($user, $chatId);
        if (! $this->isEnabled() || $messages === []) {
            return $relativePath;
        }

        $disk = Storage::disk((string) config('veda.compaction.disk', 'local'));
        $directory = dirname($relativePath);
        if (! $disk->exists($directory)) {
            $disk->makeDirectory($directory);
        }

        $lines = [];
        foreach ($messages as $message) {
            $lines[] = json_encode([
                'recorded_at' => now()->toIso8601String(),
                'role' => $message['role'] ?? null,
                'content' => $message['content'] ?? null,
                'tool_call_id' => $message['tool_call_id'] ?? null,
                'tool_calls' => $message['tool_calls'] ?? null,
            ], JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE);
        }

        $disk->append($relativePath, implode("\n", $lines)."\n");

        return $relativePath;
    }

    public function relativePath(Authenticatable $user, string $chatId): string
    {
        $safeChatId = preg_replace('/[^a-zA-Z0-9._-]/', '_', $chatId) ?: 'chat';

        return sprintf(
            'veda-transcripts/%s/%s.jsonl',
            (string) $user->getAuthIdentifier(),
            $safeChatId
        );
    }
}
