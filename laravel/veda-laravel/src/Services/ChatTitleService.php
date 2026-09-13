<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Log;

class ChatTitleService
{
    public function isPlaceholderTitle(?string $title): bool
    {
        $title = trim((string) $title);
        if ($title === '') {
            return true;
        }

        if (in_array($title, ['__NEW_CHAT__', 'New Chat'], true)) {
            return true;
        }

        return (bool) preg_match('/^(Новый чат|New Chat)(\s+\d+)?$/u', $title);
    }

    public function shouldAutoGenerate(?string $existingTitle): bool
    {
        return $this->isPlaceholderTitle($existingTitle);
    }

    public function deriveProvisionalTitle(string $userRequest): string
    {
        $text = trim(preg_replace('/\s+/u', ' ', $userRequest) ?? '');
        if ($text === '') {
            return '';
        }

        if (mb_strlen($text) > 80) {
            return rtrim(mb_substr($text, 0, 80));
        }

        return $text;
    }

    public function generateTitle(string $userRequest): string
    {
        $trimmedRequest = trim($userRequest);
        if ($trimmedRequest === '') {
            return '';
        }

        $instructions = implode("\n", [
            'You craft brief chat titles from a single user request.',
            'Return only the title text without quotes, markdown, numbering, or trailing punctuation.',
            'Maximum 8 words and 80 characters.',
            'Match the language of the user request.',
        ]);

        $provider = config('veda.title_generation.provider');
        $model = config('veda.title_generation.model');

        try {
            $response = \Laravel\Ai\agent(
                instructions: $instructions,
                messages: [],
            )->prompt(
                mb_substr($trimmedRequest, 0, 2000),
                provider: is_string($provider) && $provider !== '' ? $provider : null,
                model: is_string($model) && $model !== '' ? $model : null,
            );

            return $this->sanitizeGeneratedTitle($response->text);
        } catch (\Throwable $e) {
            Log::warning('veda.chat_title.generation_failed', [
                'error' => $e->getMessage(),
            ]);

            return '';
        }
    }

    public function sanitizeGeneratedTitle(string $rawTitle): string
    {
        $title = trim(str_replace(["\r", "\n", '"', "'", '`'], ' ', $rawTitle));
        $title = preg_replace('/\s+/u', ' ', $title);
        $title = trim((string) $title);
        if ($title === '') {
            return '';
        }

        if (preg_match('/^".*"$/u', $title)) {
            $title = trim(mb_substr($title, 1, -1));
        }

        $title = rtrim($title, " \t\n\r\0\x0B.,;:!?");
        if (mb_strlen($title) > 80) {
            $title = rtrim(mb_substr($title, 0, 80));
        }

        return $title;
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     */
    public function extractFirstUserRequest(array $messages): string
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
