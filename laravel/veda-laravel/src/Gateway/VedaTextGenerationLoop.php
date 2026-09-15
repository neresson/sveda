<?php

namespace Veda\Laravel\Gateway;

use Generator;
use Illuminate\Http\Client\RequestException;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Str;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\StepContext;
use Laravel\Ai\Gateway\TextGenerationLoop;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Messages\AssistantMessage;
use Laravel\Ai\Messages\ToolResultMessage;
use Laravel\Ai\Responses\Data\FinishReason;
use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\ToolResult;
use Laravel\Ai\Responses\Data\Usage;
use Laravel\Ai\Streaming\Events\Error;
use Laravel\Ai\Streaming\Events\StreamEnd;
use Laravel\Ai\Streaming\Events\ToolResult as ToolResultEvent;
use Laravel\Ai\Tools\ToolNameResolver;
use Throwable;
use Veda\Laravel\Services\HostMcpToolGateway;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Streaming\Events\ContextUsage;
use Veda\Laravel\Streaming\Events\MaxStepsReached;

class VedaTextGenerationLoop extends TextGenerationLoop
{
    protected bool $stoppedForFrontendToolCalls = false;

    /**
     * @param  Tool[]  $tools
     * @param  array<string, mixed>|null  $schema
     */
    public function stream(
        string $invocationId,
        TextProvider $provider,
        string $model,
        ?string $instructions,
        array $messages = [],
        array $tools = [],
        ?array $schema = null,
        ?TextGenerationOptions $options = null,
        ?int $timeout = null,
    ): Generator {
        $allMessages = $messages;
        $maxSteps = $this->resolveMaxSteps($options, $tools);
        $continuationToken = null;
        $accumulatedUsage = new Usage;
        $finalReason = null;
        $this->stoppedForFrontendToolCalls = false;

        for ($step = 0; $step < $maxSteps; $step++) {
            $tools = $this->withLatestMcpTools($tools);

            $stepContext = new StepContext(
                stepNumber: $step,
                isFinalStep: $step + 1 >= $maxSteps,
                continuationToken: $continuationToken,
            );

            $stepFailed = false;
            try {
                $stream = $this->gateway->generateStreamStep(
                    $invocationId,
                    $provider,
                    $model,
                    $instructions,
                    $allMessages,
                    $tools,
                    $schema,
                    $options,
                    $timeout,
                    $stepContext,
                );

                $stepFailed = false;
                foreach ($stream as $event) {
                    yield $event;

                    if ($event instanceof Error) {
                        $stepFailed = true;
                        $finalReason = FinishReason::Error;
                    }
                }

                $result = $stream->getReturn();
            } catch (Throwable $e) {
                report($e);
                $this->logProviderFailure($e);

                yield $this->providerErrorEvent($invocationId, $e);

                $finalReason = FinishReason::Error;
                break;
            }

            if ($stepFailed) {
                break;
            }

            if ($result !== null) {
                $accumulatedUsage = $accumulatedUsage->add($result->usage);
                $finalReason = $result->finishReason;

                yield $this->contextUsageEvent($invocationId, $result->usage);
            }

            if ($result?->finishReason === FinishReason::Continue) {
                $allMessages[] = new AssistantMessage(
                    $result->text,
                    collect($result->toolCalls),
                    $result->providerContentBlocks,
                );

                $continuationToken = $result->continuationToken;

                continue;
            }

            $toolResults = $result !== null
                ? $this->continuationToolResults($result->finishReason, $result->toolCalls, $stepContext->isFinalStep, $tools)
                : [];

            $shouldContinue = filled($toolResults);

            if ($shouldContinue) {
                foreach ($toolResults as $toolResult) {
                    yield (new ToolResultEvent(
                        strtolower((string) Str::uuid7()),
                        $toolResult,
                        true,
                        null,
                        time(),
                    ))->withInvocationId($invocationId);
                }
            }

            $allMessages[] = new AssistantMessage(
                $result?->text ?? '',
                collect($result?->toolCalls ?? []),
                $result?->providerContentBlocks ?? [],
            );

            if (! $shouldContinue) {
                break;
            }

            $allMessages[] = new ToolResultMessage(collect($toolResults));

            $continuationToken = $result?->continuationToken;
        }

        $reason = $finalReason ?? FinishReason::Error;

        if ($reason === FinishReason::ToolCalls && ! $this->stoppedForFrontendToolCalls) {
            yield (new MaxStepsReached(
                strtolower((string) Str::uuid7()),
                $maxSteps,
                $maxSteps,
                time(),
            ))->withInvocationId($invocationId);

            $reason = FinishReason::Stop;
        }

        if ($reason !== null) {
            yield (new StreamEnd(
                strtolower((string) Str::uuid7()),
                $reason->value,
                $accumulatedUsage,
                time(),
            ))->withInvocationId($invocationId);
        }
    }

    /**
     * @param  ToolCall[]  $toolCalls
     * @param  Tool[]  $tools
     * @return ToolResult[]
     */
    protected function continuationToolResults(FinishReason $reason, array $toolCalls, bool $isFinalStep, array $tools): array
    {
        if ($reason === FinishReason::ToolCalls && filled($toolCalls) && $this->hasFrontendToolCall($toolCalls, $tools)) {
            $this->stoppedForFrontendToolCalls = true;

            return [];
        }

        return parent::continuationToolResults($reason, $toolCalls, $isFinalStep, $tools);
    }

    /**
     * @param  ToolCall[]  $toolCalls
     * @param  Tool[]  $tools
     */
    protected function hasFrontendToolCall(array $toolCalls, array $tools): bool
    {
        $context = RequestContext::current();
        if ($context === null || $context->clientTools === []) {
            return false;
        }

        foreach ($toolCalls as $toolCall) {
            if (! $toolCall instanceof ToolCall) {
                continue;
            }

            if (! $context->hasClientTool($toolCall->name)) {
                continue;
            }

            if ($this->findTool($toolCall->name, $tools) !== null) {
                continue;
            }

            return true;
        }

        return false;
    }

    protected function contextUsageEvent(string $invocationId, Usage $usage): ContextUsage
    {
        $used = (int) ($usage->promptTokens ?? 0) + (int) ($usage->completionTokens ?? 0);
        $max = max(1, (int) config('veda.context_max_tokens', 128000));

        return (new ContextUsage(
            strtolower((string) Str::uuid7()),
            $used,
            $max,
            round(min(100, $used / $max * 100), 1),
            time(),
        ))->withInvocationId($invocationId);
    }

    /**
     * @param  Tool[]  $tools
     * @return Tool[]
     */
    protected function withLatestMcpTools(array $tools): array
    {
        $existing = [];
        foreach ($tools as $tool) {
            $existing[ToolNameResolver::resolve($tool)] = true;
        }

        foreach (app(HostMcpToolGateway::class)->tools(RequestContext::current()) as $tool) {
            $name = ToolNameResolver::resolve($tool);
            if (! isset($existing[$name])) {
                $tools[] = $tool;
            }
        }

        return $tools;
    }

    protected function providerErrorEvent(string $invocationId, Throwable $e): Error
    {
        return (new Error(
            strtolower((string) Str::uuid7()),
            'provider_error',
            $this->providerErrorMessage($e),
            false,
            time(),
        ))->withInvocationId($invocationId);
    }

    protected function providerErrorMessage(Throwable $e): string
    {
        if ($e instanceof RequestException && $e->response !== null) {
            $body = $e->response->json('error.message');
            if (! is_string($body) || $body === '') {
                $body = $e->response->json('message');
            }
            if (is_string($body) && $body !== '') {
                return $body;
            }
        }

        $message = trim($e->getMessage());

        return $message !== '' ? $message : 'The model provider rejected the request.';
    }

    protected function logProviderFailure(Throwable $e): void
    {
        $context = [
            'message' => $e->getMessage(),
        ];

        if ($e instanceof RequestException && $e->response !== null) {
            $context['status'] = $e->response->status();
            $context['body'] = Str::limit((string) $e->response->body(), 2000);
        }

        Log::warning('veda.stream.provider_error', $context);
    }
}
