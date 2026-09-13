<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Log;

class ConversationSummarizationService
{
    /**
     * @param  array<int, array<string, mixed>>  $toSummarize
     */
    public function summarize(array $toSummarize): string
    {
        if ($toSummarize === []) {
            return '';
        }

        $lines = [];
        foreach ($toSummarize as $message) {
            $role = (string) ($message['role'] ?? '');
            if ($role === 'user') {
                $text = (new UserMessageComposer)->extractUserRequest(MessageContent::toText($message['content'] ?? ''));
                if ($text !== '') {
                    $lines[] = 'User: '.$text;
                }
            } elseif ($role === 'assistant' && ! empty($message['tool_calls'])) {
                $names = array_values(array_filter(array_map(
                    fn ($tc) => $tc['function']['name'] ?? null,
                    $message['tool_calls']
                )));
                if ($names !== []) {
                    $lines[] = 'Assistant tools: '.implode(', ', $names);
                }
            } elseif ($role === 'tool') {
                $lines[] = 'Tool result: '.mb_substr(MessageContent::toText($message['content'] ?? ''), 0, 500, 'UTF-8');
            } elseif ($role === 'assistant') {
                $lines[] = 'Assistant: '.mb_substr(MessageContent::toText($message['content'] ?? ''), 0, 400, 'UTF-8');
            }
        }

        $provider = config('veda.compaction.provider');
        $model = config('veda.compaction.model');

        try {
            $agent = \Laravel\Ai\agent(
                instructions: 'Summarize the conversation for continuation. Preserve goals, files or entities touched, recent tool commands and results, and the next step. Use the same language as the user. Do not call tools. Output plain text only.',
                messages: [],
            );

            $response = $agent->prompt(
                implode("\n", $lines),
                provider: is_string($provider) && $provider !== '' ? $provider : null,
                model: is_string($model) && $model !== '' ? $model : null,
            );

            return trim($response->text);
        } catch (\Throwable $e) {
            Log::warning('veda.conversation_summarize.failed', [
                'error' => $e->getMessage(),
            ]);

            return '';
        }
    }
}
