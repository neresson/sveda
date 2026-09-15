<?php

namespace Veda\Laravel\Gateway;

use Generator;
use RuntimeException;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\OpenAi\OpenAiGateway;
use Laravel\Ai\Gateway\StepContext;
use Laravel\Ai\Gateway\StepResponse;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Messages\Message;
use Laravel\Ai\Messages\ToolResultMessage;
use Laravel\Ai\Messages\UserMessage;
use Laravel\Ai\Providers\Provider;
use Laravel\Ai\Responses\Data\ToolCall;
use Veda\Laravel\Gateway\Concerns\AttachesHostScreenshot;
use Veda\Laravel\Gateway\Concerns\InvokesToolsByName;
use Veda\Laravel\Gateway\Concerns\MapsActiveTools;
use Veda\Laravel\Gateway\Concerns\MapsClientTools;

class VedaResponsesGateway extends OpenAiGateway
{
    use AttachesHostScreenshot;
    use InvokesToolsByName;
    use MapsActiveTools;
    use MapsClientTools;

    protected ?Provider $activeProvider = null;

    protected string $pendingReasoningText = '';

    protected string $capturedReasoningText = '';

    protected ?string $capturedReasoningId = null;

    protected function isStateless(Provider $provider): bool
    {
        return true;
    }

    protected function isReasoningModel(string $model): bool
    {
        return false;
    }

    protected function mapTools(array $tools, Provider $provider): array
    {
        $mapped = [];
        $activeTools = $this->activeToolNames();

        foreach ($tools as $tool) {
            if ($tool instanceof Tool && $this->shouldMapTool($tool, $activeTools)) {
                $mapped[] = $this->mapTool($tool);
            }
        }

        return [...$mapped, ...$this->mapClientTools($mapped, 'responses')];
    }

    protected function buildTextRequestBody(
        Provider $provider,
        string $model,
        ?string $instructions,
        array $messages,
        array $tools,
        ?array $schema,
        ?TextGenerationOptions $options,
    ): array {
        $this->activeProvider = $provider;

        try {
            $body = parent::buildTextRequestBody($provider, $model, $instructions, $messages, $tools, $schema, $options);
        } finally {
            $this->activeProvider = null;
        }

        if (($body['tools'] ?? []) === []) {
            $clientTools = $this->mapClientTools([], 'responses');
            if ($clientTools !== []) {
                $body['tools'] = $clientTools;
                $body['tool_choice'] = 'auto';
            }
        }

        if (isset($body['input']) && is_array($body['input'])) {
            $body['input'] = $this->pairFunctionCallOutputs($body['input']);
            $body['input'] = $this->replayReasoningText($body['input']);
            $body['input'] = $this->ensureReasoningTextForToolCalls(
                $body['input'],
                is_string($body['reasoning']['effort'] ?? null) ? $body['reasoning']['effort'] : null,
            );
        }

        return $body;
    }

    public function generateStreamStep(
        string $invocationId,
        TextProvider $provider,
        string $model,
        ?string $instructions,
        array $messages,
        array $tools,
        ?array $schema,
        ?TextGenerationOptions $options,
        ?int $timeout,
        StepContext $stepContext,
    ): Generator {
        $this->pendingReasoningText = '';

        $body = $this->buildStepBody($provider, $model, $instructions, $messages, $tools, $schema, $options, $stepContext);
        $body['stream'] = true;
        $idleTimeout = $this->streamIdleTimeout($timeout);

        $response = $this->withErrorHandling(
            $provider->name(),
            fn () => $this->client($provider, $timeout)
                ->connectTimeout(min(15, $idleTimeout))
                ->withOptions(['stream' => true])
                ->post('responses', $body),
        );

        $streamBody = $response->getBody();
        $this->applyIdleTimeout($streamBody, $idleTimeout);

        $stream = $this->processTextStream($invocationId, $provider, $model, $streamBody);

        foreach ($stream as $event) {
            yield $event;
        }

        $result = $stream->getReturn();
        if ($result instanceof StepResponse) {
            $result = $this->attachCapturedReasoning($result);
        }

        return $result;
    }

    protected function parseServerSentEvents($streamBody): Generator
    {
        $this->pendingReasoningText = '';

        while (! $streamBody->eof()) {
            $line = trim($this->readLine($streamBody));

            if ($this->streamTimedOut($streamBody)) {
                throw new RuntimeException('The model stream stalled before it finished.');
            }

            if ($line === '') {
                continue;
            }

            $payload = $this->extractSseDataPayload($line);
            if ($payload === null) {
                continue;
            }

            if ($payload === '[DONE]') {
                return;
            }

            $decoded = json_decode($payload, true);
            if (json_last_error() !== JSON_ERROR_NONE || ! is_array($decoded)) {
                continue;
            }

            $type = is_string($decoded['type'] ?? null) ? $decoded['type'] : '';
            $normalized = $this->normalizeResponsesStreamEvent($decoded);
            if ($normalized !== null) {
                yield $normalized;
            }

            if (in_array($type, ['response.completed', 'response.incomplete', 'response.failed', 'error'], true)) {
                return;
            }
        }
    }

    protected function readLine($streamBody): string
    {
        $buffer = '';

        while (! $streamBody->eof()) {
            $byte = $streamBody->read(1);

            if ($byte === '') {
                if ($this->streamTimedOut($streamBody)) {
                    throw new RuntimeException('The model stream stalled before it finished.');
                }

                if ($buffer !== '' || $streamBody->eof()) {
                    return $buffer;
                }

                usleep(20000);

                continue;
            }

            $buffer .= $byte;

            if ($byte === "\n") {
                break;
            }
        }

        return $buffer;
    }

    protected function extractSseDataPayload(string $line): ?string
    {
        if (str_starts_with($line, 'data:')) {
            return trim(substr($line, 5));
        }

        if (str_starts_with($line, 'event:')) {
            $pos = stripos($line, 'data:');
            if ($pos !== false) {
                return trim(substr($line, $pos + 5));
            }
        }

        return null;
    }

    protected function streamIdleTimeout(?int $timeout): int
    {
        $configured = max(5, (int) config('veda.stream_idle_timeout', 90));

        if ($timeout === null || $timeout <= 0) {
            return $configured;
        }

        return max(5, min($configured, $timeout));
    }

    protected function applyIdleTimeout(mixed $streamBody, int $seconds): void
    {
        if (! is_object($streamBody) || ! method_exists($streamBody, 'getMetadata')) {
            return;
        }

        $resource = $streamBody->getMetadata('stream');
        if (is_resource($resource)) {
            stream_set_timeout($resource, max(1, $seconds));
        }
    }

    protected function streamTimedOut(mixed $streamBody): bool
    {
        if (! is_object($streamBody) || ! method_exists($streamBody, 'getMetadata')) {
            return false;
        }

        if ($streamBody->getMetadata('timed_out') === true) {
            return true;
        }

        $resource = $streamBody->getMetadata('stream');
        if (is_resource($resource)) {
            return (bool) (stream_get_meta_data($resource)['timed_out'] ?? false);
        }

        return false;
    }

    /**
     * @param  array<string, mixed>  $data
     * @return array<string, mixed>|null
     */
    protected function normalizeResponsesStreamEvent(array $data): ?array
    {
        $type = $data['type'] ?? '';

        if ($type === 'response.created') {
            $this->capturedReasoningText = '';
            $this->capturedReasoningId = null;
            $this->pendingReasoningText = '';
        }

        if ($type === 'response.reasoning_text.delta' || $type === 'response.reasoning.delta') {
            $this->pendingReasoningText .= (string) ($data['delta'] ?? '');
            $this->capturedReasoningText = $this->pendingReasoningText;
            $data['type'] = 'response.reasoning_summary_text.delta';

            return $data;
        }

        if ($type === 'response.reasoning_text.done' || $type === 'response.reasoning.done') {
            $text = $data['text'] ?? null;
            if (is_string($text) && $text !== '') {
                $this->pendingReasoningText = $text;
                $this->capturedReasoningText = $text;
            }

            return null;
        }

        if ($type === 'response.output_item.added' && ($data['item']['type'] ?? null) === 'reasoning') {
            $id = $data['item']['id'] ?? null;
            if (is_string($id) && $id !== '') {
                $this->capturedReasoningId = $id;
            }
        }

        if ($type === 'response.output_item.done' && ($data['item']['type'] ?? null) === 'reasoning') {
            $item = is_array($data['item'] ?? null) ? $data['item'] : [];

            $id = $item['id'] ?? null;
            if (is_string($id) && $id !== '') {
                $this->capturedReasoningId = $id;
            }

            if (! $this->hasReasoningTextParts($item['content'] ?? null) && $this->pendingReasoningText !== '') {
                $item['content'] = [
                    ['type' => 'reasoning_text', 'text' => $this->pendingReasoningText],
                ];
            }

            if (! $this->hasReasoningTextParts($item['summary'] ?? null) && $this->hasReasoningTextParts($item['content'] ?? null)) {
                $item['summary'] = $item['content'];
            }

            $fromItem = $this->extractReasoningText($item);
            if ($fromItem !== '') {
                $this->capturedReasoningText = $fromItem;
            }

            $data['item'] = $item;
            $this->pendingReasoningText = '';
        }

        return $data;
    }

    /**
     * @param  array<int, array<string, mixed>>  $input
     * @return array<int, array<string, mixed>>
     */
    protected function pairFunctionCallOutputs(array $input): array
    {
        $outputsByCallId = [];
        $outputsWithoutCallId = [];
        $others = [];

        foreach ($input as $item) {
            if (! is_array($item) || ($item['type'] ?? null) !== 'function_call_output') {
                $others[] = $item;

                continue;
            }

            $callId = $item['call_id'] ?? null;
            if (is_string($callId) && $callId !== '') {
                $outputsByCallId[$callId] = $item;
            } else {
                $outputsWithoutCallId[] = $item;
            }
        }

        $paired = [];
        foreach ($others as $item) {
            if (! is_array($item) || ($item['type'] ?? null) !== 'function_call') {
                $paired[] = $item;

                continue;
            }

            $callId = $item['call_id'] ?? null;
            if (! is_string($callId) || $callId === '') {
                $fallbackId = $item['id'] ?? null;
                if (is_string($fallbackId) && $fallbackId !== '') {
                    $callId = $fallbackId;
                    $item['call_id'] = $callId;
                }
            }

            $paired[] = $item;

            if (is_string($callId) && isset($outputsByCallId[$callId])) {
                $paired[] = $outputsByCallId[$callId];
                unset($outputsByCallId[$callId]);

                continue;
            }

            if ($outputsWithoutCallId !== []) {
                $output = array_shift($outputsWithoutCallId);
                if (is_string($callId) && $callId !== '') {
                    $output['call_id'] = $callId;
                }
                $paired[] = $output;
            }
        }

        foreach ($outputsByCallId as $output) {
            $paired[] = $output;
        }

        return [...$paired, ...$outputsWithoutCallId];
    }

    /**
     * @param  array<int, mixed>  $input
     * @return array<int, mixed>
     */
    protected function replayReasoningText(array $input): array
    {
        $replayed = [];

        foreach ($input as $item) {
            if (! is_array($item) || ($item['type'] ?? null) !== 'reasoning') {
                $replayed[] = $item;

                continue;
            }

            $text = $this->extractReasoningText($item);
            if ($text === '') {
                continue;
            }

            $replayed[] = [
                'type' => 'reasoning',
                'content' => [
                    ['type' => 'reasoning_text', 'text' => $text],
                ],
            ];
        }

        return $replayed;
    }

    /**
     * @param  array<int, mixed>  $input
     * @return array<int, mixed>
     */
    protected function ensureReasoningTextForToolCalls(array $input, ?string $effort): array
    {
        if ($effort === null || $effort === 'none') {
            return $this->placeReasoningBeforeFunctionCalls($input);
        }

        $hasFunctionCall = false;
        $hasReasoningText = false;

        foreach ($input as $item) {
            if (! is_array($item)) {
                continue;
            }

            if (($item['type'] ?? null) === 'function_call') {
                $hasFunctionCall = true;
            }

            if (($item['type'] ?? null) === 'reasoning' && $this->extractReasoningText($item) !== '') {
                $hasReasoningText = true;
            }
        }

        if ($hasFunctionCall && ! $hasReasoningText) {
            $text = $this->capturedReasoningText !== '' ? $this->capturedReasoningText : '.';
            array_unshift($input, [
                'type' => 'reasoning',
                'content' => [
                    ['type' => 'reasoning_text', 'text' => $text],
                ],
            ]);
        }

        return $this->placeReasoningBeforeFunctionCalls($input);
    }

    /**
     * @param  array<int, mixed>  $input
     * @return array<int, mixed>
     */
    protected function placeReasoningBeforeFunctionCalls(array $input): array
    {
        $reasoning = null;
        $rest = [];

        foreach ($input as $item) {
            if (is_array($item) && ($item['type'] ?? null) === 'reasoning') {
                $reasoning = $item;

                continue;
            }

            $rest[] = $item;
        }

        if ($reasoning === null) {
            return $input;
        }

        $placed = [];
        $used = false;

        foreach ($rest as $item) {
            if (is_array($item) && ($item['type'] ?? null) === 'function_call') {
                $placed[] = $reasoning;
                $used = true;
            }

            $placed[] = $item;
        }

        if (! $used) {
            $placed[] = $reasoning;
        }

        return $placed;
    }

    protected function attachCapturedReasoning(StepResponse $result): StepResponse
    {
        $text = $this->capturedReasoningText;
        if ($text === '' || $result->toolCalls === []) {
            return $result;
        }

        $reasoningId = $this->capturedReasoningId;
        $summary = [
            ['type' => 'reasoning_text', 'text' => $text],
        ];

        $toolCalls = array_map(function (ToolCall $call) use ($reasoningId, $summary): ToolCall {
            $existing = $this->textFromReasoningParts($call->reasoningSummary);
            if ($existing !== '') {
                if ($call->reasoningId) {
                    return $call;
                }

                return new ToolCall(
                    $call->id,
                    $call->name,
                    $call->arguments,
                    $call->resultId,
                    $reasoningId ?? 'rs_veda',
                    $call->reasoningSummary,
                    $call->reasoningEncryptedContent,
                );
            }

            return new ToolCall(
                $call->id,
                $call->name,
                $call->arguments,
                $call->resultId,
                $call->reasoningId ?? $reasoningId ?? 'rs_veda',
                $summary,
                $call->reasoningEncryptedContent,
            );
        }, $result->toolCalls);

        return new StepResponse(
            text: $result->text,
            toolCalls: $toolCalls,
            finishReason: $result->finishReason,
            usage: $result->usage,
            meta: $result->meta,
            structured: $result->structured,
            continuationToken: $result->continuationToken,
            providerContentBlocks: $result->providerContentBlocks,
        );
    }

    /**
     * @param  array<string, mixed>  $item
     */
    protected function extractReasoningText(array $item): string
    {
        foreach (['content', 'summary'] as $key) {
            $text = $this->textFromReasoningParts($item[$key] ?? null);
            if ($text !== '') {
                return $text;
            }
        }

        return '';
    }

    protected function hasReasoningTextParts(mixed $parts): bool
    {
        return $this->textFromReasoningParts($parts) !== '';
    }

    protected function textFromReasoningParts(mixed $parts): string
    {
        if (! is_array($parts)) {
            return '';
        }

        $chunks = [];
        foreach ($parts as $part) {
            if (! is_array($part)) {
                continue;
            }

            $type = $part['type'] ?? null;
            $text = $part['text'] ?? null;
            if (! is_string($text) || $text === '') {
                continue;
            }

            if ($type !== null && ! in_array($type, ['reasoning_text', 'summary_text'], true)) {
                continue;
            }

            $chunks[] = $text;
        }

        return implode('', $chunks);
    }

    protected function mapToolResultMessage(ToolResultMessage|Message $message, array &$input): void
    {
        if (! $message instanceof ToolResultMessage) {
            return;
        }

        foreach ($message->toolResults as $toolResult) {
            $callId = $toolResult->resultId ?: $toolResult->id;

            $input[] = [
                'type' => 'function_call_output',
                'call_id' => $callId,
                'output' => $this->serializeToolResultOutput($toolResult->result),
            ];
        }
    }

    protected function mapUserMessage(UserMessage|Message $message, array &$input, ?Provider $provider = null): void
    {
        $provider ??= $this->activeProvider;

        $isEmpty = trim((string) $message->content) === ''
            && (! $message instanceof UserMessage || $message->attachments->isEmpty());

        if ($isEmpty && ! $this->hasHostScreenshot()) {
            return;
        }

        $parent = new \ReflectionMethod(parent::class, 'mapUserMessage');
        if ($parent->getNumberOfParameters() >= 3) {
            parent::mapUserMessage($message, $input, $provider);
        } else {
            parent::mapUserMessage($message, $input);
        }

        $this->appendResponsesScreenshot($input, $provider);
    }
}
