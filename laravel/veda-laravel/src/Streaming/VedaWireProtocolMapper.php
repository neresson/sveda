<?php

namespace Veda\Laravel\Streaming;

use Laravel\Ai\Streaming\Events\Error;
use Laravel\Ai\Streaming\Events\ReasoningDelta;
use Laravel\Ai\Streaming\Events\StreamEnd;
use Laravel\Ai\Streaming\Events\StreamEvent;
use Laravel\Ai\Streaming\Events\StreamStart;
use Laravel\Ai\Streaming\Events\TextDelta;
use Laravel\Ai\Streaming\Events\ToolCall;
use Laravel\Ai\Streaming\Events\ToolResult;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Streaming\Events\ContextUsage;
use Veda\Laravel\Streaming\Events\MaxStepsReached;

class VedaWireProtocolMapper
{
    public const VERSION = '1.0';

    public function __construct(
        protected ?string $chatId = null,
        protected ?string $messageId = null,
    ) {}

    /**
     * @return array<string, mixed>|null
     */
    public function map(StreamEvent $event): ?array
    {
        $mapped = match (true) {
            $event instanceof StreamStart => ['type' => 'message.start'],
            $event instanceof TextDelta => [
                'type' => 'text.delta',
                'delta' => $event->delta,
            ],
            $event instanceof ReasoningDelta => [
                'type' => 'reasoning.delta',
                'delta' => $event->delta,
            ],
            $event instanceof ToolCall => [
                'type' => 'tool.call',
                'toolCallId' => $event->toolCall->id,
                'toolName' => $event->toolCall->name,
                'target' => $this->toolTarget($event->toolCall->name),
                'input' => $event->toolCall->arguments,
            ],
            $event instanceof ToolResult => $this->mapToolResult($event),
            $event instanceof ContextUsage => [
                'type' => 'context.usage',
                'usedTokens' => $event->usedTokens,
                'maxTokens' => $event->maxTokens,
                'percent' => $event->percent,
            ],
            $event instanceof MaxStepsReached => [
                'type' => 'max_steps',
                'maxSteps' => $event->maxSteps,
                'canContinue' => true,
            ],
            $event instanceof StreamEnd => [
                'type' => 'message.end',
                'finishReason' => $event->reason,
                'usage' => [
                    'promptTokens' => $event->usage->promptTokens ?? 0,
                    'completionTokens' => $event->usage->completionTokens ?? 0,
                    'totalTokens' => ($event->usage->promptTokens ?? 0) + ($event->usage->completionTokens ?? 0),
                ],
            ],
            $event instanceof Error => [
                'type' => 'error',
                'code' => $event->type,
                'message' => $event->message,
            ],
            default => null,
        };

        if ($mapped === null) {
            return null;
        }

        if ($this->chatId !== null) {
            $mapped['chatId'] = $this->chatId;
        }
        if ($this->messageId !== null) {
            $mapped['messageId'] = $this->messageId;
        }
        $mapped['timestamp'] = now()->toIso8601String();

        return $mapped;
    }

    protected function toolTarget(string $toolName): string
    {
        $context = RequestContext::current();

        return $context !== null && $context->hasClientTool($toolName) ? 'frontend' : 'backend';
    }

    /**
     * @return array<string, mixed>
     */
    protected function mapToolResult(ToolResult $event): array
    {
        $output = $this->normalizeOutput($event->toolResult->result);

        $mapped = [
            'type' => 'tool.result',
            'toolCallId' => $event->toolResult->resultId ?? $event->toolResult->id,
            'toolName' => $event->toolResult->name,
            'output' => $output,
        ];

        $links = $this->extractResourceLinks($output);
        if ($links !== null) {
            $mapped['renderHint'] = 'resource_links';
            $mapped['renderData'] = ['links' => $links];
        }

        return $mapped;
    }

    /**
     * @return array<int, array<string, mixed>>|null
     */
    protected function extractResourceLinks(mixed $output): ?array
    {
        if (! is_array($output)) {
            return null;
        }

        $links = $output['links'] ?? null;
        if (! is_array($links) || $links === []) {
            return null;
        }

        $normalized = [];
        foreach ($links as $link) {
            if (! is_array($link)) {
                continue;
            }

            $label = $link['label'] ?? $link['name'] ?? null;
            $url = $link['url'] ?? $link['link'] ?? null;
            if (! is_string($label) || trim($label) === '' || ! is_string($url) || trim($url) === '') {
                continue;
            }

            $item = ['label' => trim($label), 'url' => trim($url)];
            if (is_string($link['icon'] ?? null) && trim($link['icon']) !== '') {
                $item['icon'] = trim($link['icon']);
            }
            if (is_string($link['kind'] ?? null) && trim($link['kind']) !== '') {
                $item['kind'] = trim($link['kind']);
            }

            $normalized[] = $item;
        }

        return $normalized === [] ? null : $normalized;
    }

    protected function normalizeOutput(mixed $result): mixed
    {
        if (is_string($result)) {
            $decoded = json_decode($result, true);

            return $decoded ?? $result;
        }

        return $result;
    }
}
