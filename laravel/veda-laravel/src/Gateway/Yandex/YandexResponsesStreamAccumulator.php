<?php

namespace Veda\Laravel\Gateway\Yandex;

final class YandexResponsesStreamAccumulator
{
    protected string $content = '';

    protected string $reasoning = '';

    /** @var array<int, array<string, mixed>> */
    protected array $toolCalls = [];

    /** @var array<string, int> */
    protected array $toolCallKeyByCallId = [];

    protected ?string $finishReason = null;

    protected int $tokensUsed = 0;

    protected int $promptTokens = 0;

    protected int $completionTokens = 0;

    public function content(): string
    {
        return $this->content;
    }

    public function reasoning(): string
    {
        return $this->reasoning;
    }

    public function applyEvent(string $eventType, array $data): void
    {
        if ($this->looksLikeCompletedResponse($data)) {
            $this->applyCompletedResponse($data);

            return;
        }

        if (isset($data['response']) && is_array($data['response'])) {
            $this->applyCompletedResponse($data['response']);

            return;
        }

        $resolvedType = (string) ($data['type'] ?? $eventType);

        if ($resolvedType === 'response.output_text.delta' || $resolvedType === 'response.text.delta') {
            $this->appendTextDelta((string) ($data['delta'] ?? $data['text'] ?? ''));

            return;
        }

        if ($resolvedType === 'response.function_call_arguments.delta') {
            $callId = (string) ($data['call_id'] ?? '');
            $delta = (string) ($data['delta'] ?? '');
            if ($callId === '' || $delta === '') {
                return;
            }

            $key = $this->toolCallKeyByCallId[$callId] ?? null;
            if ($key === null) {
                $key = count($this->toolCalls);
                $this->toolCalls[$key] = [
                    'id' => $callId,
                    'type' => 'function',
                    'function' => [
                        'name' => (string) ($data['name'] ?? ''),
                        'arguments' => '',
                    ],
                ];
                $this->toolCallKeyByCallId[$callId] = $key;
            }

            $this->toolCalls[$key]['function']['arguments'] .= $delta;

            return;
        }

        if (str_contains($resolvedType, 'delta') && ! str_contains($resolvedType, 'function_call')) {
            $this->appendTextDelta((string) ($data['delta'] ?? $data['text'] ?? $data['output_text'] ?? ''));

            return;
        }

        if ($resolvedType === 'response.reasoning_summary_text.delta') {
            $delta = (string) ($data['delta'] ?? '');
            if ($delta !== '') {
                $this->reasoning .= $delta;
            }

            return;
        }

        if ($resolvedType === 'response.output_item.added' || $resolvedType === 'response.output_item.done') {
            $item = $data['item'] ?? null;
            if (is_array($item)) {
                $this->applyOutputItem($item);
            }

            return;
        }

        if ($resolvedType === 'response.completed' || $resolvedType === 'response.done' || $resolvedType === 'response.incomplete') {
            $response = $data['response'] ?? $data;
            if (is_array($response)) {
                $this->applyCompletedResponse($response);
            }
        }
    }

    public function applyRawStreamBody(string $body): void
    {
        $trimmed = trim($body);
        if ($trimmed === '') {
            return;
        }

        $decoded = json_decode($trimmed, true);
        if (is_array($decoded)) {
            $this->applyEvent('', $decoded);

            return;
        }

        foreach (explode("\n", $trimmed) as $line) {
            $line = trim($line);
            if ($line === '' || ! str_starts_with($line, 'data:')) {
                continue;
            }

            $payload = trim(substr($line, 5));
            if ($payload === '' || $payload === '[DONE]') {
                continue;
            }

            $data = json_decode($payload, true);
            if (is_array($data)) {
                $this->applyEvent('', $data);
            }
        }
    }

    /**
     * @param  array<string, mixed>  $response
     */
    public function applyCompletedResponse(array $response): void
    {
        if (isset($response['error']) && is_array($response['error'])) {
            $message = (string) ($response['error']['message'] ?? json_encode($response['error'], JSON_UNESCAPED_UNICODE));

            throw new \RuntimeException('Yandex Responses API error: '.$message);
        }

        $output = $response['output'] ?? [];
        if (is_array($output)) {
            foreach ($output as $item) {
                if (is_array($item) && (string) ($item['type'] ?? '') === 'function_call') {
                    $this->registerFunctionCallItem($item);
                }
            }
        }

        $this->mergeTextContent($this->resolveAuthoritativeTextFromResponse($response));

        $this->applyUsage($response['usage'] ?? null);
        $this->finishReason = 'stop';
    }

    /**
     * @param  array<string, mixed>  $item
     */
    protected function applyOutputItem(array $item): void
    {
        $type = (string) ($item['type'] ?? '');

        if ($type === 'function_call') {
            $this->registerFunctionCallItem($item);

            return;
        }

        if ($type !== 'message') {
            return;
        }

        $this->mergeTextContent($this->extractMessageItemText($item));
    }

    /**
     * @param  array<string, mixed>  $data
     */
    protected function looksLikeCompletedResponse(array $data): bool
    {
        if (isset($data['output']) || isset($data['output_text'])) {
            return true;
        }

        $status = (string) ($data['status'] ?? '');

        return in_array($status, ['completed', 'incomplete', 'failed'], true);
    }

    protected function appendTextDelta(string $delta): void
    {
        if ($delta === '') {
            return;
        }

        if ($this->content !== '' && str_ends_with($this->content, $delta)) {
            return;
        }

        if ($this->content !== '' && str_starts_with($delta, $this->content)) {
            $this->content = $delta;

            return;
        }

        $this->content .= $delta;
    }

    protected function mergeTextContent(string $next): void
    {
        $next = trim($next);
        if ($next === '') {
            return;
        }

        $current = $this->content;
        if ($current === '') {
            $this->content = $next;

            return;
        }

        if ($current === $next) {
            return;
        }

        if (str_starts_with($next, $current)) {
            $this->content = $next;

            return;
        }

        if (str_contains($next, $current)) {
            $this->content = $next;

            return;
        }

        if (str_contains($current, $next) || str_ends_with($current, $next)) {
            return;
        }

        if (str_contains($current, $next.$next)) {
            $this->content = $next;

            return;
        }

        $this->content .= $next;
    }

    /**
     * @param  array<string, mixed>  $response
     */
    protected function resolveAuthoritativeTextFromResponse(array $response): string
    {
        $candidates = [];

        $outputText = trim((string) ($response['output_text'] ?? ''));
        if ($outputText !== '') {
            $candidates[] = $outputText;
        }

        $output = $response['output'] ?? [];
        if (is_array($output)) {
            foreach ($output as $item) {
                if (! is_array($item) || (string) ($item['type'] ?? '') !== 'message') {
                    continue;
                }

                $text = $this->extractMessageItemText($item);
                if ($text !== '') {
                    $candidates[] = $text;
                }
            }
        }

        $choices = $response['choices'] ?? null;
        if (is_array($choices) && isset($choices[0]['message']['content'])) {
            $legacy = trim((string) $choices[0]['message']['content']);
            if ($legacy !== '') {
                $candidates[] = $legacy;
            }
        }

        return $this->pickLongestCandidate($candidates);
    }

    /**
     * @param  array<string, mixed>  $item
     */
    protected function extractMessageItemText(array $item): string
    {
        $content = $item['content'] ?? null;
        if (is_string($content)) {
            return trim($content);
        }

        if (! is_array($content)) {
            return '';
        }

        $parts = [];
        foreach ($content as $part) {
            if (! is_array($part)) {
                continue;
            }

            $partType = (string) ($part['type'] ?? '');
            if (! in_array($partType, ['output_text', 'text'], true)) {
                continue;
            }

            $text = trim((string) ($part['text'] ?? ''));
            if ($text !== '') {
                $parts[] = $text;
            }
        }

        return implode('', $parts);
    }

    /**
     * @param  array<int, string>  $candidates
     */
    protected function pickLongestCandidate(array $candidates): string
    {
        $candidates = array_values(array_filter(array_map('trim', $candidates), fn ($candidate) => $candidate !== ''));
        if ($candidates === []) {
            return '';
        }

        usort($candidates, fn (string $a, string $b): int => strlen($b) <=> strlen($a));

        return $candidates[0];
    }

    /**
     * @param  array<string, mixed>  $item
     */
    protected function registerFunctionCallItem(array $item): void
    {
        $callId = (string) ($item['call_id'] ?? $item['id'] ?? '');
        $name = (string) ($item['name'] ?? '');
        $arguments = (string) ($item['arguments'] ?? '{}');
        if ($callId === '' || $name === '') {
            return;
        }

        $key = $this->toolCallKeyByCallId[$callId] ?? count($this->toolCalls);
        $this->toolCallKeyByCallId[$callId] = $key;
        $existingArgs = (string) ($this->toolCalls[$key]['function']['arguments'] ?? '');
        $this->toolCalls[$key] = [
            'id' => $callId,
            'type' => 'function',
            'function' => [
                'name' => $name,
                'arguments' => $existingArgs !== '' ? $existingArgs : $arguments,
            ],
        ];
    }

    protected function applyUsage(mixed $usage): void
    {
        if (! is_array($usage)) {
            return;
        }

        if (isset($usage['total_tokens'])) {
            $this->tokensUsed = max($this->tokensUsed, (int) $usage['total_tokens']);
        }
        if (isset($usage['input_tokens'])) {
            $this->promptTokens = max($this->promptTokens, (int) $usage['input_tokens']);
        }
        if (isset($usage['output_tokens'])) {
            $this->completionTokens = max($this->completionTokens, (int) $usage['output_tokens']);
        }
    }

    /**
     * @return array<string, mixed>
     */
    public function toStreamResult(): array
    {
        $message = [
            'role' => 'assistant',
            'content' => $this->content,
            'tool_calls' => [],
        ];

        $toolCalls = array_values($this->toolCalls);
        if ($toolCalls !== []) {
            $message['tool_calls'] = $toolCalls;
            unset($message['content']);
            $message['reasoning_content'] = $this->reasoning;
        }

        if ($this->tokensUsed === 0 && $this->completionTokens > 0) {
            $this->tokensUsed = $this->promptTokens + $this->completionTokens;
        }

        return [
            'message' => $message,
            'content' => $this->content,
            'tool_calls' => $toolCalls,
            'finish_reason' => $this->finishReason ?? 'stop',
            'tokens_used' => $this->tokensUsed,
            'prompt_tokens' => $this->promptTokens,
            'completion_tokens' => $this->completionTokens,
            'prompt_cache_tokens' => 0,
        ];
    }
}
