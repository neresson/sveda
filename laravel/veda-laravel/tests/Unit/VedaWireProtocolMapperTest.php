<?php

namespace Veda\Laravel\Tests\Unit;

use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\ToolResult;
use Laravel\Ai\Responses\Data\Usage;
use Laravel\Ai\Streaming\Events\StreamEnd;
use Laravel\Ai\Streaming\Events\StreamStart;
use Laravel\Ai\Streaming\Events\TextDelta;
use Laravel\Ai\Streaming\Events\ToolCall as ToolCallEvent;
use Laravel\Ai\Streaming\Events\ToolResult as ToolResultEvent;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Streaming\Events\ContextUsage;
use Veda\Laravel\Streaming\Events\MaxStepsReached;
use Veda\Laravel\Streaming\VedaWireProtocolMapper;
use Veda\Laravel\Tests\TestCase;

class VedaWireProtocolMapperTest extends TestCase
{
    public function test_maps_stream_start_to_message_start(): void
    {
        $mapper = new VedaWireProtocolMapper('chat-1');

        $mapped = $mapper->map(new StreamStart('inv-1', 'openai', 'gpt-5', time()));

        $this->assertSame('message.start', $mapped['type']);
        $this->assertSame('chat-1', $mapped['chatId']);
        $this->assertArrayHasKey('timestamp', $mapped);
    }

    public function test_maps_text_delta(): void
    {
        $mapper = new VedaWireProtocolMapper('chat-1', 'msg-1');

        $mapped = $mapper->map(new TextDelta('inv-1', 'msg-1', 'Hello', time()));

        $this->assertSame('text.delta', $mapped['type']);
        $this->assertSame('Hello', $mapped['delta']);
        $this->assertSame('msg-1', $mapped['messageId']);
    }

    public function test_maps_tool_call_and_result(): void
    {
        $mapper = new VedaWireProtocolMapper('chat-1');

        $toolCall = new ToolCall('call-1', 'get_weather', ['city' => 'Berlin']);
        $mappedCall = $mapper->map(new ToolCallEvent('inv-1', $toolCall, time()));

        $this->assertSame('tool.call', $mappedCall['type']);
        $this->assertSame('call-1', $mappedCall['toolCallId']);
        $this->assertSame('get_weather', $mappedCall['toolName']);
        $this->assertSame('backend', $mappedCall['target']);
        $this->assertSame(['city' => 'Berlin'], $mappedCall['input']);

        $toolResult = new ToolResult('call-1', 'get_weather', ['city' => 'Berlin'], '{"success":true}');
        $mappedResult = $mapper->map(new ToolResultEvent('inv-1', $toolResult, true, null, time()));

        $this->assertSame('tool.result', $mappedResult['type']);
        $this->assertSame('call-1', $mappedResult['toolCallId']);
        $this->assertSame(['success' => true], $mappedResult['output']);
    }

    public function test_maps_stream_end_to_message_end(): void
    {
        $mapper = new VedaWireProtocolMapper('chat-1');

        $mapped = $mapper->map(new StreamEnd('inv-1', 'stop', new Usage(10, 20), time()));

        $this->assertSame('message.end', $mapped['type']);
        $this->assertSame('stop', $mapped['finishReason']);
        $this->assertSame(10, $mapped['usage']['promptTokens']);
        $this->assertSame(20, $mapped['usage']['completionTokens']);
        $this->assertSame(30, $mapped['usage']['totalTokens']);
    }

    public function test_maps_max_steps_reached(): void
    {
        $mapper = new VedaWireProtocolMapper('chat-1');

        $mapped = $mapper->map(new MaxStepsReached('inv-1', 12, 0, time()));

        $this->assertSame('max_steps', $mapped['type']);
        $this->assertSame(12, $mapped['maxSteps']);
        $this->assertTrue($mapped['canContinue']);
    }

    public function test_marks_client_tool_calls_as_frontend_target(): void
    {
        RequestContext::bind([], false, clientTools: [
            ['name' => 'confirm_course_creation', 'description' => 'Confirm', 'parameters' => ['type' => 'object']],
        ]);

        try {
            $mapper = new VedaWireProtocolMapper('chat-1');

            $frontend = $mapper->map(new ToolCallEvent(
                'inv-1',
                new ToolCall('call-1', 'confirm_course_creation', ['title' => 'Math']),
                time(),
            ));
            $this->assertSame('frontend', $frontend['target']);

            $backend = $mapper->map(new ToolCallEvent(
                'inv-1',
                new ToolCall('call-2', 'search_knowledge_base', ['query' => 'x']),
                time(),
            ));
            $this->assertSame('backend', $backend['target']);
        } finally {
            RequestContext::forget();
        }
    }

    public function test_tool_result_carries_render_hint_from_output_links(): void
    {
        $mapper = new VedaWireProtocolMapper('chat-1');

        $withLinks = $mapper->map(new ToolResultEvent(
            'inv-1',
            new ToolResult('call-1', 'create_course', [], [
                'success' => true,
                'links' => [
                    ['label' => 'Math', 'url' => '/teacher/courses/1', 'kind' => 'created'],
                    ['name' => 'Ignored'],
                ],
            ]),
            true,
            null,
            time(),
        ));

        $this->assertSame('resource_links', $withLinks['renderHint']);
        $this->assertSame(
            ['links' => [['label' => 'Math', 'url' => '/teacher/courses/1', 'kind' => 'created']]],
            $withLinks['renderData'],
        );

        $withoutLinks = $mapper->map(new ToolResultEvent(
            'inv-1',
            new ToolResult('call-2', 'search', [], ['success' => true]),
            true,
            null,
            time(),
        ));

        $this->assertArrayNotHasKey('renderHint', $withoutLinks);
        $this->assertArrayNotHasKey('renderData', $withoutLinks);
    }

    public function test_maps_context_usage(): void
    {
        $mapper = new VedaWireProtocolMapper('chat-1');

        $mapped = $mapper->map(new ContextUsage('inv-1', 500, 128000, 0.4, time()));

        $this->assertSame('context.usage', $mapped['type']);
        $this->assertSame(500, $mapped['usedTokens']);
        $this->assertSame(128000, $mapped['maxTokens']);
        $this->assertSame(0.4, $mapped['percent']);
    }
}
