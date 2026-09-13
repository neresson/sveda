<?php

namespace Veda\Laravel\Gateway\Yandex;

use Veda\Laravel\Gateway\ProviderSession;

final class YandexResponsesPayloadBuilder
{
    public function __construct(protected YandexAiConfig $config) {}

    /**
     * @param  array<string, mixed>  $chatPayload
     * @return array<string, mixed>
     */
    public function fromChatPayload(ProviderSession $session, array $chatPayload): array
    {
        $messages = $chatPayload['messages'] ?? [];
        if (! is_array($messages)) {
            $messages = [];
        }

        [$instructions, $conversationMessages] = $this->splitSystemMessages($messages);
        $input = $this->buildInput($conversationMessages);

        $payload = [
            'model' => $this->config->modelUri($session->model),
            'input' => $input,
            'stream' => (bool) ($chatPayload['stream'] ?? false),
            'parallel_tool_calls' => true,
            'reasoning' => ['effort' => 'none'],
        ];

        if ($instructions !== '') {
            $payload['instructions'] = $instructions;
        }

        if (isset($chatPayload['tools']) && is_array($chatPayload['tools']) && $chatPayload['tools'] !== []) {
            $payload['tools'] = $this->normalizeTools($chatPayload['tools']);
        }

        if (isset($chatPayload['tool_choice'])) {
            $payload['tool_choice'] = $this->normalizeToolChoice($chatPayload['tool_choice']);
        }

        $maxTokens = $chatPayload['max_output_tokens']
            ?? $chatPayload['max_completion_tokens']
            ?? null;
        if (is_numeric($maxTokens)) {
            $payload['max_output_tokens'] = (int) $maxTokens;
        }

        if (isset($chatPayload['temperature']) && is_numeric($chatPayload['temperature'])) {
            $payload['temperature'] = (float) $chatPayload['temperature'];
        }

        return $payload;
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     * @return array{0: string, 1: array<int, array<string, mixed>>}
     */
    protected function splitSystemMessages(array $messages): array
    {
        $instructions = [];
        $rest = [];

        foreach ($messages as $message) {
            if (($message['role'] ?? '') === 'system') {
                $text = $this->messageText($message['content'] ?? '');
                if ($text !== '') {
                    $instructions[] = $text;
                }

                continue;
            }

            $rest[] = $message;
        }

        return [implode("\n\n", $instructions), $rest];
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     * @return array<int, mixed>|string
     */
    protected function buildInput(array $messages): array|string
    {
        $input = [];

        foreach ($messages as $message) {
            $role = (string) ($message['role'] ?? '');

            if ($role === 'user') {
                $parts = $this->userContentParts($message['content'] ?? '');
                if ($parts !== []) {
                    $input[] = [
                        'type' => 'message',
                        'role' => 'user',
                        'content' => $parts,
                    ];
                }

                continue;
            }

            if ($role === 'assistant') {
                $text = trim($this->messageText($message['content'] ?? ''));
                if ($text !== '') {
                    $input[] = [
                        'type' => 'message',
                        'role' => 'assistant',
                        'content' => [
                            ['type' => 'output_text', 'text' => $text],
                        ],
                    ];
                }

                $toolCalls = $message['tool_calls'] ?? [];
                if (is_array($toolCalls)) {
                    foreach ($toolCalls as $toolCall) {
                        $callId = (string) ($toolCall['id'] ?? '');
                        $name = (string) ($toolCall['function']['name'] ?? '');
                        $arguments = (string) ($toolCall['function']['arguments'] ?? '{}');
                        if ($callId === '' || $name === '') {
                            continue;
                        }

                        $input[] = [
                            'type' => 'function_call',
                            'call_id' => $callId,
                            'name' => $name,
                            'arguments' => $arguments,
                        ];
                    }
                }

                continue;
            }

            if ($role === 'tool') {
                $callId = (string) ($message['tool_call_id'] ?? '');
                if ($callId === '') {
                    continue;
                }

                $input[] = [
                    'type' => 'function_call_output',
                    'call_id' => $callId,
                    'output' => (string) ($message['content'] ?? ''),
                ];
            }
        }

        if ($input === []) {
            return '';
        }

        return $input;
    }

    /**
     * @return array<int, array<string, mixed>>
     */
    protected function userContentParts(mixed $content): array
    {
        if (is_string($content)) {
            $text = trim($content);

            return $text === '' ? [] : [['type' => 'input_text', 'text' => $text]];
        }

        if (! is_array($content)) {
            return [];
        }

        $parts = [];
        foreach ($content as $part) {
            if (! is_array($part)) {
                continue;
            }

            $type = (string) ($part['type'] ?? '');
            if ($type === 'text') {
                $text = trim((string) ($part['text'] ?? ''));
                if ($text !== '') {
                    $parts[] = ['type' => 'input_text', 'text' => $text];
                }

                continue;
            }

            if ($type === 'image_url') {
                $url = (string) ($part['image_url']['url'] ?? $part['image_url'] ?? '');
                if ($url !== '') {
                    $parts[] = [
                        'type' => 'input_image',
                        'image_url' => $url,
                        'detail' => (string) ($part['image_url']['detail'] ?? $part['detail'] ?? 'auto'),
                    ];
                }
            }
        }

        return $parts;
    }

    protected function messageText(mixed $content): string
    {
        if (is_string($content)) {
            return $content;
        }

        if (! is_array($content)) {
            return '';
        }

        $chunks = [];
        foreach ($content as $part) {
            if (is_array($part) && ($part['type'] ?? '') === 'text') {
                $chunks[] = (string) ($part['text'] ?? '');
            }
        }

        return implode("\n", array_filter($chunks, fn ($chunk) => $chunk !== ''));
    }

    /**
     * @param  array<int, mixed>  $tools
     * @return array<int, array<string, mixed>>
     */
    protected function normalizeTools(array $tools): array
    {
        $normalized = [];

        foreach ($tools as $tool) {
            if (! is_array($tool)) {
                continue;
            }

            $converted = $this->normalizeToolDefinition($tool);
            if ($converted !== null) {
                $normalized[] = $converted;
            }
        }

        return $normalized;
    }

    /**
     * @param  array<string, mixed>  $tool
     * @return array<string, mixed>|null
     */
    protected function normalizeToolDefinition(array $tool): ?array
    {
        $function = $tool['function'] ?? null;
        if (is_array($function)) {
            $name = trim((string) ($function['name'] ?? ''));
            if ($name === '') {
                return null;
            }

            $normalized = [
                'type' => 'function',
                'name' => $name,
            ];

            $description = trim((string) ($function['description'] ?? ''));
            if ($description !== '') {
                $normalized['description'] = $description;
            }

            if (isset($function['parameters']) && is_array($function['parameters'])) {
                $normalized['parameters'] = $function['parameters'];
            }

            if (array_key_exists('strict', $function)) {
                $normalized['strict'] = $function['strict'];
            }

            return $normalized;
        }

        $name = trim((string) ($tool['name'] ?? ''));
        if ($name === '') {
            return null;
        }

        $normalized = [
            'type' => (string) ($tool['type'] ?? 'function'),
            'name' => $name,
        ];

        $description = trim((string) ($tool['description'] ?? ''));
        if ($description !== '') {
            $normalized['description'] = $description;
        }

        if (isset($tool['parameters']) && is_array($tool['parameters'])) {
            $normalized['parameters'] = $tool['parameters'];
        }

        if (array_key_exists('strict', $tool)) {
            $normalized['strict'] = $tool['strict'];
        }

        return $normalized;
    }

    protected function normalizeToolChoice(mixed $toolChoice): mixed
    {
        if (! is_array($toolChoice)) {
            return $toolChoice;
        }

        if (($toolChoice['type'] ?? '') !== 'function') {
            return $toolChoice;
        }

        $function = $toolChoice['function'] ?? null;
        if (! is_array($function)) {
            return $toolChoice;
        }

        $name = trim((string) ($function['name'] ?? ''));
        if ($name === '') {
            return $toolChoice;
        }

        return [
            'type' => 'function',
            'name' => $name,
        ];
    }
}
