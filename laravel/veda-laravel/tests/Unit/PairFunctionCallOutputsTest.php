<?php

namespace Veda\Laravel\Tests\Unit;

use Illuminate\Contracts\Events\Dispatcher;
use Laravel\Ai\Gateway\StepResponse;
use Laravel\Ai\Responses\Data\FinishReason;
use Laravel\Ai\Responses\Data\Meta;
use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\Usage;
use ReflectionMethod;
use ReflectionProperty;
use Veda\Laravel\Gateway\VedaResponsesGateway;
use Veda\Laravel\Tests\TestCase;

class PairFunctionCallOutputsTest extends TestCase
{
    public function test_places_each_tool_output_immediately_after_its_call(): void
    {
        $paired = $this->pair([
            ['role' => 'user', 'content' => 'hi'],
            ['type' => 'function_call', 'id' => 'fc1', 'call_id' => 'call_1', 'name' => 'search', 'arguments' => '{}'],
            ['type' => 'function_call', 'id' => 'fc2', 'call_id' => 'call_2', 'name' => 'search', 'arguments' => '{}'],
            ['type' => 'function_call_output', 'call_id' => 'call_1', 'output' => 'one'],
            ['type' => 'function_call_output', 'call_id' => 'call_2', 'output' => 'two'],
        ]);

        $this->assertSame('user', $paired[0]['role']);
        $this->assertSame('function_call', $paired[1]['type']);
        $this->assertSame('call_1', $paired[1]['call_id']);
        $this->assertSame('function_call_output', $paired[2]['type']);
        $this->assertSame('call_1', $paired[2]['call_id']);
        $this->assertSame('function_call', $paired[3]['type']);
        $this->assertSame('call_2', $paired[3]['call_id']);
        $this->assertSame('function_call_output', $paired[4]['type']);
        $this->assertSame('call_2', $paired[4]['call_id']);
    }

    public function test_fills_missing_output_call_id_from_the_function_call(): void
    {
        $paired = $this->pair([
            ['type' => 'function_call', 'id' => 'fc1', 'call_id' => 'call_1', 'name' => 'search', 'arguments' => '{}'],
            ['type' => 'function_call_output', 'call_id' => null, 'output' => 'one'],
        ]);

        $this->assertSame('function_call_output', $paired[1]['type']);
        $this->assertSame('call_1', $paired[1]['call_id']);
    }

    public function test_replays_summary_as_plaintext_reasoning_text(): void
    {
        $replayed = $this->replay([
            ['role' => 'user', 'content' => 'hi'],
            ['type' => 'reasoning', 'id' => 'r1', 'summary' => [], 'encrypted_content' => 'abc'],
            ['type' => 'function_call', 'call_id' => 'call_1', 'name' => 'search', 'arguments' => '{}'],
            [
                'type' => 'reasoning',
                'id' => 'r2',
                'summary' => [['type' => 'summary_text', 'text' => 'need cottage cheese']],
            ],
        ]);

        $this->assertCount(3, $replayed);
        $this->assertSame('user', $replayed[0]['role']);
        $this->assertSame('function_call', $replayed[1]['type']);
        $this->assertSame('reasoning', $replayed[2]['type']);
        $this->assertArrayNotHasKey('id', $replayed[2]);
        $this->assertSame(
            [['type' => 'reasoning_text', 'text' => 'need cottage cheese']],
            $replayed[2]['content'],
        );
        $this->assertArrayNotHasKey('summary', $replayed[2]);
        $this->assertArrayNotHasKey('encrypted_content', $replayed[2]);
    }

    public function test_keeps_existing_reasoning_text_content(): void
    {
        $replayed = $this->replay([
            [
                'type' => 'reasoning',
                'id' => 'r1',
                'content' => [['type' => 'reasoning_text', 'text' => 'thinking']],
            ],
        ]);

        $this->assertSame('thinking', $replayed[0]['content'][0]['text']);
    }

    public function test_aliases_reasoning_text_stream_events_onto_summary_deltas(): void
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $method = new ReflectionMethod($gateway, 'normalizeResponsesStreamEvent');

        $delta = $method->invoke($gateway, [
            'type' => 'response.reasoning_text.delta',
            'delta' => 'need ',
        ]);
        $this->assertSame('response.reasoning_summary_text.delta', $delta['type']);
        $this->assertSame('need ', $delta['delta']);

        $done = $method->invoke($gateway, [
            'type' => 'response.reasoning_text.done',
            'text' => 'need cottage cheese',
        ]);
        $this->assertNull($done);

        $item = $method->invoke($gateway, [
            'type' => 'response.output_item.done',
            'item' => [
                'type' => 'reasoning',
                'id' => 'rs_1',
                'summary' => [],
            ],
        ]);

        $this->assertSame('reasoning_text', $item['item']['content'][0]['type']);
        $this->assertSame('need cottage cheese', $item['item']['content'][0]['text']);
        $this->assertSame($item['item']['content'], $item['item']['summary']);
    }

    public function test_attaches_captured_reasoning_to_tool_calls_without_it(): void
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $gatewayCapture = new ReflectionProperty($gateway, 'capturedReasoningText');
        $gatewayCapture->setValue($gateway, 'need cottage cheese');
        $id = new ReflectionProperty($gateway, 'capturedReasoningId');
        $id->setValue($gateway, 'rs_1');

        $method = new ReflectionMethod($gateway, 'attachCapturedReasoning');
        $attached = $method->invoke($gateway, new StepResponse(
            'searching',
            [new ToolCall('fc1', 'search', ['q' => 'tvorog'], 'call_1')],
            FinishReason::ToolCalls,
            new Usage(1, 1),
            new Meta('veda-responses', 'deepseek-v4-flash'),
        ));

        $this->assertSame('rs_1', $attached->toolCalls[0]->reasoningId);
        $this->assertSame('need cottage cheese', $attached->toolCalls[0]->reasoningSummary[0]['text']);
    }

    public function test_places_reasoning_immediately_before_each_function_call(): void
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $method = new ReflectionMethod($gateway, 'placeReasoningBeforeFunctionCalls');

        $placed = $method->invoke($gateway, [
            ['role' => 'user', 'content' => 'hi'],
            [
                'type' => 'reasoning',
                'content' => [['type' => 'reasoning_text', 'text' => 'think']],
            ],
            ['type' => 'function_call', 'call_id' => 'c1', 'name' => 'search', 'arguments' => '{}'],
            ['type' => 'function_call_output', 'call_id' => 'c1', 'output' => 'one'],
            ['type' => 'function_call', 'call_id' => 'c2', 'name' => 'search', 'arguments' => '{}'],
            ['type' => 'function_call_output', 'call_id' => 'c2', 'output' => 'two'],
        ]);

        $this->assertSame('reasoning', $placed[1]['type']);
        $this->assertSame('function_call', $placed[2]['type']);
        $this->assertSame('c1', $placed[2]['call_id']);
        $this->assertSame('function_call_output', $placed[3]['type']);
        $this->assertSame('reasoning', $placed[4]['type']);
        $this->assertSame('function_call', $placed[5]['type']);
        $this->assertSame('c2', $placed[5]['call_id']);
    }

    public function test_injects_placeholder_reasoning_when_thinking_has_no_text(): void
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $method = new ReflectionMethod($gateway, 'ensureReasoningTextForToolCalls');

        $ensured = $method->invoke($gateway, [
            ['role' => 'user', 'content' => 'hi'],
            ['type' => 'function_call', 'call_id' => 'c1', 'name' => 'search', 'arguments' => '{}'],
            ['type' => 'function_call_output', 'call_id' => 'c1', 'output' => 'one'],
        ], 'high');

        $this->assertSame('reasoning', $ensured[1]['type']);
        $this->assertSame('.', $ensured[1]['content'][0]['text']);
        $this->assertSame('function_call', $ensured[2]['type']);
    }

    /**
     * @param  array<int, array<string, mixed>>  $input
     * @return array<int, array<string, mixed>>
     */
    protected function pair(array $input): array
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $method = new ReflectionMethod($gateway, 'pairFunctionCallOutputs');

        return $method->invoke($gateway, $input);
    }

    /**
     * @param  array<int, array<string, mixed>>  $input
     * @return array<int, array<string, mixed>>
     */
    protected function replay(array $input): array
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $method = new ReflectionMethod($gateway, 'replayReasoningText');

        return $method->invoke($gateway, $input);
    }
}
