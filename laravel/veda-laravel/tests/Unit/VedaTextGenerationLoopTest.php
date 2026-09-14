<?php

namespace Veda\Laravel\Tests\Unit;

use Generator;
use Laravel\Ai\Contracts\Gateway\StepTextGateway;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Gateway\StepContext;
use Laravel\Ai\Gateway\StepResponse;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Responses\Data\FinishReason;
use Laravel\Ai\Responses\Data\Meta;
use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\Usage;
use Laravel\Ai\Streaming\Events\StreamEnd;
use Laravel\Ai\Streaming\Events\ToolCall as ToolCallEvent;
use Laravel\Ai\Streaming\Events\ToolResult as ToolResultEvent;
use Mockery;
use Veda\Laravel\Gateway\VedaTextGenerationLoop;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Streaming\Events\ContextUsage;
use Veda\Laravel\Streaming\Events\MaxStepsReached;
use Veda\Laravel\Tests\Fixtures\DummyReadTool;
use Veda\Laravel\Tests\TestCase;

class VedaTextGenerationLoopTest extends TestCase
{
    protected function tearDown(): void
    {
        RequestContext::forget();

        parent::tearDown();
    }

    public function test_frontend_tool_call_stops_loop_without_execution_or_max_steps(): void
    {
        RequestContext::bind([], false, clientTools: [
            ['name' => 'confirm_course_creation', 'description' => 'Confirm', 'parameters' => ['type' => 'object']],
        ]);

        $toolCall = new ToolCall('call-1', 'confirm_course_creation', ['title' => 'Math']);
        $gateway = new LoopFakeGateway([
            new StepResponse('', [$toolCall], FinishReason::ToolCalls, new Usage(10, 5), new Meta('deepseek', 'deepseek-v4-flash')),
        ]);

        $events = $this->collectEvents($gateway);

        $this->assertSame(1, $gateway->streamCalls);

        $toolCallEvents = array_values(array_filter($events, fn ($event) => $event instanceof ToolCallEvent));
        $this->assertCount(1, $toolCallEvents);

        $this->assertEmpty(array_filter($events, fn ($event) => $event instanceof ToolResultEvent));
        $this->assertEmpty(array_filter($events, fn ($event) => $event instanceof MaxStepsReached));

        $streamEnd = end($events);
        $this->assertInstanceOf(StreamEnd::class, $streamEnd);
        $this->assertSame(FinishReason::ToolCalls->value, $streamEnd->reason);
    }

    public function test_backend_tool_call_on_final_step_emits_max_steps_reached(): void
    {
        $toolCall = new ToolCall('call-1', 'unknown_backend_tool', ['x' => 1]);
        $gateway = new LoopFakeGateway([
            new StepResponse('', [$toolCall], FinishReason::ToolCalls, new Usage(10, 5), new Meta('deepseek', 'deepseek-v4-flash')),
        ]);

        $events = $this->collectEvents($gateway, [], new TextGenerationOptions(maxSteps: 1));

        $maxSteps = array_values(array_filter($events, fn ($event) => $event instanceof MaxStepsReached));
        $this->assertCount(1, $maxSteps);

        $streamEnd = end($events);
        $this->assertInstanceOf(StreamEnd::class, $streamEnd);
        $this->assertSame(FinishReason::Stop->value, $streamEnd->reason);
    }

    public function test_backend_tool_executes_and_loop_continues(): void
    {
        $toolCall = new ToolCall('call-1', 'dummy_read', ['query' => 'abc']);
        $gateway = new LoopFakeGateway([
            new StepResponse('', [$toolCall], FinishReason::ToolCalls, new Usage(10, 5), new Meta('deepseek', 'deepseek-v4-flash')),
            new StepResponse('Done', [], FinishReason::Stop, new Usage(8, 4), new Meta('deepseek', 'deepseek-v4-flash')),
        ]);

        $events = $this->collectEvents($gateway, [new DummyReadTool]);

        $this->assertSame(2, $gateway->streamCalls);

        $toolResults = array_values(array_filter($events, fn ($event) => $event instanceof ToolResultEvent));
        $this->assertCount(1, $toolResults);
        $this->assertSame('dummy_read', $toolResults[0]->toolResult->name);

        $streamEnd = end($events);
        $this->assertInstanceOf(StreamEnd::class, $streamEnd);
        $this->assertSame(FinishReason::Stop->value, $streamEnd->reason);
    }

    public function test_context_usage_event_is_emitted_per_step(): void
    {
        config()->set('veda.context_max_tokens', 1000);

        $gateway = new LoopFakeGateway([
            new StepResponse('Hi', [], FinishReason::Stop, new Usage(100, 50), new Meta('deepseek', 'deepseek-v4-flash')),
        ]);

        $events = $this->collectEvents($gateway);

        $usageEvents = array_values(array_filter($events, fn ($event) => $event instanceof ContextUsage));
        $this->assertCount(1, $usageEvents);
        $this->assertSame(150, $usageEvents[0]->usedTokens);
        $this->assertSame(1000, $usageEvents[0]->maxTokens);
        $this->assertSame(15.0, $usageEvents[0]->percent);
    }

    public function test_generate_does_not_throw_for_frontend_tool_calls(): void
    {
        RequestContext::bind([], false, clientTools: [
            ['name' => 'confirm_course_creation', 'description' => 'Confirm', 'parameters' => ['type' => 'object']],
        ]);

        $gateway = new LoopFakeGateway([
            new StepResponse('', [new ToolCall('call-1', 'confirm_course_creation', [])], FinishReason::ToolCalls, new Usage(1, 1), new Meta('deepseek', 'deepseek-v4-flash')),
        ]);

        $loop = new VedaTextGenerationLoop($gateway);
        $response = $loop->generate($this->fakeProvider(), 'model', null, [], []);

        $this->assertSame('call-1', $response->toolCalls->first()->id);
    }

    /**
     * @param  array<int, object>  $tools
     * @return array<int, object>
     */
    protected function collectEvents(LoopFakeGateway $gateway, array $tools = [], ?TextGenerationOptions $options = null): array
    {
        $loop = new VedaTextGenerationLoop($gateway);

        return iterator_to_array($loop->stream(
            'inv-1',
            $this->fakeProvider(),
            'model',
            null,
            [],
            $tools,
            null,
            $options,
        ), false);
    }

    protected function fakeProvider(): TextProvider
    {
        $provider = Mockery::mock(TextProvider::class);
        $provider->allows('name')->andReturn('deepseek');
        $provider->allows('driver')->andReturn('veda-responses');

        return $provider;
    }
}

class LoopFakeGateway implements StepTextGateway
{
    public int $streamCalls = 0;

    /**
     * @param  array<int, StepResponse>  $steps
     */
    public function __construct(protected array $steps) {}

    public function generateTextStep(
        TextProvider $provider,
        string $model,
        ?string $instructions,
        array $messages,
        array $tools,
        ?array $schema,
        ?TextGenerationOptions $options,
        ?int $timeout,
        StepContext $stepContext,
    ): StepResponse {
        return $this->steps[$stepContext->stepNumber] ?? end($this->steps);
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
        $this->streamCalls++;

        $step = $this->steps[$stepContext->stepNumber] ?? end($this->steps);

        foreach ($step->toolCalls as $toolCall) {
            yield (new ToolCallEvent('evt-'.$toolCall->id, $toolCall, time()))->withInvocationId($invocationId);
        }

        return $step;
    }
}
