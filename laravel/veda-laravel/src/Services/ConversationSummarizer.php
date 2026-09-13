<?php

namespace Veda\Laravel\Services;

class ConversationSummarizer
{
    public const RATIO_START_BACKGROUND = 0.80;

    public const RATIO_BLOCK_AND_APPLY = 0.95;

    /**
     * @param  array<int, array<string, mixed>>  $messages
     */
    public function usageRatio(array $messages, int $maxChars): float
    {
        if ($maxChars <= 0) {
            return 0.0;
        }

        $json = json_encode($messages, JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE);

        return is_string($json) ? strlen($json) / $maxChars : 0.0;
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     * @return array{messages: array<int, array<string, mixed>>, applied: bool, summary: string}
     */
    public function applyIfNeeded(
        array $messages,
        int $maxChars,
        ?callable $llmSummarize = null
    ): array {
        $ratio = $this->usageRatio($messages, $maxChars);
        if ($ratio < self::RATIO_START_BACKGROUND) {
            return ['messages' => $messages, 'applied' => false, 'summary' => ''];
        }

        $toSummarize = $this->selectMessagesBeforeAnchor($messages);
        if ($toSummarize === []) {
            return ['messages' => $messages, 'applied' => false, 'summary' => ''];
        }

        $summary = '';
        if ($llmSummarize !== null) {
            $summary = trim((string) $llmSummarize($toSummarize));
        }
        if ($summary === '') {
            $summary = $this->buildExtractiveSummary($toSummarize);
        }

        return [
            'messages' => $this->buildCompactedMessages($messages, $summary),
            'applied' => true,
            'summary' => $summary,
        ];
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     * @return array<int, array<string, mixed>>
     */
    public function buildCompactedMessages(array $messages, string $summary, ?string $transcriptPath = null): array
    {
        $toSummarize = $this->selectMessagesBeforeAnchor($messages);
        $summaryBlock = $this->formatSummaryBlock($summary, $transcriptPath);

        $system = [];
        $pinnedUser = null;
        $tail = [];
        $foundAnchor = false;

        foreach ($messages as $message) {
            if (($message['role'] ?? '') === 'system') {
                $system[] = $message;

                continue;
            }
            if ($pinnedUser === null && ($message['role'] ?? '') === 'user') {
                $pinnedUser = $message;

                continue;
            }
            if (! $foundAnchor && $this->messageInList($message, $toSummarize)) {
                $foundAnchor = true;

                continue;
            }
            if ($foundAnchor) {
                $tail[] = $message;
            }
        }

        $compact = $system;
        $compact[] = [
            'role' => 'assistant',
            'content' => $summaryBlock,
        ];
        if ($pinnedUser !== null) {
            $compact[] = $pinnedUser;
        }
        foreach ($tail as $message) {
            $compact[] = $message;
        }

        return $compact;
    }

    public function formatSummaryBlock(string $summary, ?string $transcriptPath = null): string
    {
        $lines = [trim($summary)];
        if (is_string($transcriptPath) && $transcriptPath !== '') {
            $lines[] = '';
            $lines[] = 'Full transcript: '.$transcriptPath;
        }

        return "<conversation_summary>\n".implode("\n", $lines)."\n</conversation_summary>";
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     * @return array<int, array<string, mixed>>
     */
    public function selectMessagesBeforeAnchor(array $messages): array
    {
        $nonSystem = array_values(array_filter($messages, fn ($m) => ($m['role'] ?? '') !== 'system'));
        if (count($nonSystem) <= 2) {
            return [];
        }

        $keepFrom = max(0, count($nonSystem) - 2);

        return array_slice($nonSystem, 0, $keepFrom);
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     */
    public function buildExtractiveSummary(array $messages): string
    {
        $sections = [
            'Overview: Earlier conversation compacted to preserve context window.',
            'Recent operations:',
        ];

        foreach ($messages as $message) {
            $role = (string) ($message['role'] ?? '');
            if ($role === 'user') {
                $text = (new UserMessageComposer)->extractUserRequest(MessageContent::toText($message['content'] ?? ''));
                $preview = mb_substr(trim($text), 0, 400, 'UTF-8');
                if ($preview !== '') {
                    $sections[] = '- User: '.$preview;
                }

                continue;
            }
            if ($role === 'assistant') {
                if (! empty($message['tool_calls'])) {
                    $names = array_values(array_filter(array_map(
                        fn ($tc) => $tc['function']['name'] ?? null,
                        $message['tool_calls']
                    )));
                    if ($names !== []) {
                        $sections[] = '- Assistant called: '.implode(', ', $names);
                    }

                    continue;
                }
                $preview = mb_substr(trim(MessageContent::toText($message['content'] ?? '')), 0, 300, 'UTF-8');
                if ($preview !== '') {
                    $sections[] = '- Assistant: '.$preview;
                }

                continue;
            }
            if ($role === 'tool') {
                $preview = mb_substr(trim(MessageContent::toText($message['content'] ?? '')), 0, 200, 'UTF-8');
                if ($preview !== '') {
                    $sections[] = '- Tool result: '.$preview;
                }
            }
        }

        $sections[] = 'Continuation: Use the summary plus recent messages below. Re-fetch context or run searches if details are missing.';

        return implode("\n", $sections);
    }

    /**
     * @param  array<string, mixed>  $message
     * @param  array<int, array<string, mixed>>  $list
     */
    protected function messageInList(array $message, array $list): bool
    {
        foreach ($list as $candidate) {
            if ($candidate === $message) {
                return true;
            }
            if (
                ($candidate['role'] ?? null) === ($message['role'] ?? null)
                && ($candidate['content'] ?? null) === ($message['content'] ?? null)
                && ($candidate['tool_call_id'] ?? null) === ($message['tool_call_id'] ?? null)
            ) {
                return true;
            }
        }

        return false;
    }
}
