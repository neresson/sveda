<?php

namespace Veda\Laravel\Tests\Unit;

use Generator;
use GuzzleHttp\Psr7\Response as PsrResponse;
use Illuminate\Http\Client\Request;
use Illuminate\Http\Client\RequestException;
use Illuminate\Http\Client\Response as HttpResponse;
use Illuminate\Support\Facades\Http;
use Laravel\Ai\Contracts\Gateway\StepTextGateway;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Gateway\StepContext;
use Laravel\Ai\Gateway\StepResponse;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Responses\Data\FinishReason;
use Laravel\Ai\Responses\Data\Meta;
use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\Usage;
use Laravel\Ai\Streaming\Events\Error;
use Laravel\Ai\Streaming\Events\StreamEnd;
use Laravel\Ai\Streaming\Events\ToolCall as ToolCallEvent;
use Laravel\Ai\Streaming\Events\ToolResult as ToolResultEvent;
use Laravel\Ai\Tools\ToolNameResolver;
use Mockery;
use Veda\Laravel\Gateway\VedaTextGenerationLoop;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Streaming\Events\ContextUsage;
use Veda\Laravel\Streaming\Events\MaxStepsReached;
use Veda\Laravel\Tests\Fixtures\DummyReadTool;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\ManageMcpCatalogTool;

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

    public function test_provider_exception_emits_error_and_stream_end(): void
    {
        $gateway = new LoopFakeGateway([], throwOnStep: 0);

        $events = $this->collectEvents($gateway);

        $errors = array_values(array_filter($events, fn ($event) => $event instanceof Error));
        $this->assertCount(1, $errors);
        $this->assertSame('Invalid schema', $errors[0]->message);

        $streamEnd = end($events);
        $this->assertInstanceOf(StreamEnd::class, $streamEnd);
        $this->assertSame(FinishReason::Error->value, $streamEnd->reason);
    }

    public function test_connected_mcp_tools_are_available_on_the_next_step(): void
    {
        Http::preventStrayRequests();
        Http::fake(function (Request $request) {
            if ($request->method() === 'DELETE') {
                return Http::response('', 200);
            }

            $payload = json_decode($request->body(), true);
            $method = is_array($payload) ? ($payload['method'] ?? '') : '';
            $id = is_array($payload) ? ($payload['id'] ?? 1) : 1;

            return match ($method) {
                'initialize' => Http::response([
                    'jsonrpc' => '2.0',
                    'id' => $id,
                    'result' => [
                        'protocolVersion' => '2025-11-25',
                        'capabilities' => ['tools' => ['listChanged' => false]],
                        'serverInfo' => ['name' => 'vkusvill', 'version' => '0.1.0'],
                    ],
                ], 200, [
                    'Content-Type' => 'application/json',
                    'MCP-Session-Id' => 'sess-test',
                ]),
                'notifications/initialized' => Http::response('', 202),
                'tools/list' => Http::response([
                    'jsonrpc' => '2.0',
                    'id' => $id,
                    'result' => [
                        'tools' => [[
                            'name' => 'search_products',
                            'title' => 'search_products',
                            'description' => 'Search products',
                            'inputSchema' => ['type' => 'object', 'properties' => (object) []],
                            'annotations' => [],
                            '_meta' => ['domain' => 'other', 'mode' => 'read'],
                        ]],
                    ],
                ]),
                default => Http::response('', 202),
            };
        });

        $toolCall = new ToolCall('call-1', ManageMcpCatalogTool::NAME, [
            'action' => 'connect',
            'url' => 'https://mcp.vkusvill.ru/mcp',
        ]);
        $gateway = new LoopFakeGateway([
            new StepResponse('', [$toolCall], FinishReason::ToolCalls, new Usage(10, 5), new Meta('deepseek', 'deepseek-v4-flash')),
            new StepResponse('Done', [], FinishReason::Stop, new Usage(8, 4), new Meta('deepseek', 'deepseek-v4-flash')),
        ]);

        $this->collectEvents($gateway, [new ManageMcpCatalogTool]);

        $this->assertSame(2, $gateway->streamCalls);
        $this->assertContains(ManageMcpCatalogTool::NAME, $gateway->toolNamesByStep[0] ?? []);
        $this->assertContains('vkusvill__search_products', $gateway->toolNamesByStep[1] ?? []);
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
     * @var array<int, array<int, string>>
     */
    public array $toolNamesByStep = [];

    /**
     * @param  array<int, StepResponse>  $steps
     */
    public function __construct(protected array $steps, public ?int $throwOnStep = null) {}

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
        $this->toolNamesByStep[] = array_map(
            fn ($tool): string => ToolNameResolver::resolve($tool),
            $tools,
        );

        if ($this->throwOnStep !== null && $stepContext->stepNumber === $this->throwOnStep) {
            throw new RequestException(new HttpResponse(new PsrResponse(
                400,
                ['Content-Type' => 'application/json'],
                json_encode(['error' => ['message' => 'Invalid schema']]),
            )));
        }

        $step = $this->steps[$stepContext->stepNumber] ?? end($this->steps);

        foreach ($step->toolCalls as $toolCall) {
            yield (new ToolCallEvent('evt-'.$toolCall->id, $toolCall, time()))->withInvocationId($invocationId);
        }

        return $step;
    }
}
