<?php

namespace Veda\Laravel\Streaming;

class VercelDataProtocolMapper
{
    /**
     * @param  array<string, mixed>  $event
     * @return array<string, mixed>|null
     */
    public function map(array $event): ?array
    {
        $type = $event['type'] ?? null;
        if (! is_string($type)) {
            return null;
        }

        return match ($type) {
            'text.delta' => [
                'type' => 'text-delta',
                'textDelta' => (string) ($event['delta'] ?? ''),
            ],
            'reasoning.delta' => [
                'type' => 'reasoning-delta',
                'reasoningDelta' => (string) ($event['delta'] ?? ''),
            ],
            'tool.call' => [
                'type' => 'tool-call',
                'toolCallId' => (string) ($event['toolCallId'] ?? ''),
                'toolName' => (string) ($event['toolName'] ?? ''),
                'args' => is_array($event['input'] ?? null) ? $event['input'] : [],
                'target' => ($event['target'] ?? 'backend') === 'frontend' ? 'frontend' : 'backend',
            ],
            'tool.result' => array_filter([
                'type' => 'tool-result',
                'toolCallId' => (string) ($event['toolCallId'] ?? ''),
                'toolName' => (string) ($event['toolName'] ?? ''),
                'result' => $event['output'] ?? null,
                'renderHint' => $event['renderHint'] ?? null,
                'renderData' => $event['renderData'] ?? null,
            ], fn (mixed $value) => $value !== null),
            'tool.progress' => [
                'type' => 'data-toolProgress',
                'data' => [
                    'phase' => $event['phase'] ?? null,
                    'tasks' => is_array($event['tasks'] ?? null) ? $event['tasks'] : [],
                ],
            ],
            'context.usage' => [
                'type' => 'data-contextUsage',
                'data' => [
                    'usedTokens' => (int) ($event['usedTokens'] ?? 0),
                    'maxTokens' => (int) ($event['maxTokens'] ?? 0),
                    'percent' => (float) ($event['percent'] ?? 0),
                ],
            ],
            'chat.title' => [
                'type' => 'data-chatTitle',
                'data' => ['title' => (string) ($event['title'] ?? '')],
            ],
            'max_steps' => [
                'type' => 'data-maxStepsReached',
                'data' => [
                    'maxSteps' => (int) ($event['maxSteps'] ?? 0),
                    'canContinue' => (bool) ($event['canContinue'] ?? true),
                ],
            ],
            'message.end' => [
                'type' => 'finish',
                'finishReason' => (string) ($event['finishReason'] ?? 'stop'),
                'usage' => $event['usage'] ?? null,
            ],
            'error' => [
                'type' => 'error',
                'error' => [
                    'code' => (string) ($event['code'] ?? 'unknown'),
                    'message' => (string) ($event['message'] ?? 'Unknown error'),
                ],
            ],
            default => null,
        };
    }
}
