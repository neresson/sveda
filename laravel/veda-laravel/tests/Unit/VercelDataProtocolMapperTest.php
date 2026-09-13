<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Streaming\VercelDataProtocolMapper;
use Veda\Laravel\Tests\TestCase;

class VercelDataProtocolMapperTest extends TestCase
{
    public function test_maps_text_delta(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertSame(
            ['type' => 'text-delta', 'textDelta' => 'Hello'],
            $mapper->map(['type' => 'text.delta', 'delta' => 'Hello'])
        );
    }

    public function test_maps_reasoning_delta(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertSame(
            ['type' => 'reasoning-delta', 'reasoningDelta' => 'thinking'],
            $mapper->map(['type' => 'reasoning.delta', 'delta' => 'thinking'])
        );
    }

    public function test_maps_tool_call_with_frontend_target(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertSame(
            [
                'type' => 'tool-call',
                'toolCallId' => 'call-1',
                'toolName' => 'confirm_course_creation',
                'args' => ['title' => 'Math'],
                'target' => 'frontend',
            ],
            $mapper->map([
                'type' => 'tool.call',
                'toolCallId' => 'call-1',
                'toolName' => 'confirm_course_creation',
                'input' => ['title' => 'Math'],
                'target' => 'frontend',
            ])
        );
    }

    public function test_maps_tool_result_with_render_hint(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertSame(
            [
                'type' => 'tool-result',
                'toolCallId' => 'call-1',
                'toolName' => 'create_course',
                'result' => ['success' => true],
                'renderHint' => 'resource_links',
                'renderData' => ['links' => [['label' => 'Math', 'url' => '/teacher/courses/1']]],
            ],
            $mapper->map([
                'type' => 'tool.result',
                'toolCallId' => 'call-1',
                'toolName' => 'create_course',
                'output' => ['success' => true],
                'renderHint' => 'resource_links',
                'renderData' => ['links' => [['label' => 'Math', 'url' => '/teacher/courses/1']]],
            ])
        );
    }

    public function test_maps_tool_result_without_render_hint_omits_keys(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $mapped = $mapper->map([
            'type' => 'tool.result',
            'toolCallId' => 'call-1',
            'toolName' => 'search',
            'output' => ['success' => true],
        ]);

        $this->assertArrayNotHasKey('renderHint', $mapped);
        $this->assertArrayNotHasKey('renderData', $mapped);
    }

    public function test_maps_custom_data_events(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertSame(
            ['type' => 'data-toolProgress', 'data' => ['phase' => 'search', 'tasks' => [['id' => 't1']]]],
            $mapper->map(['type' => 'tool.progress', 'phase' => 'search', 'tasks' => [['id' => 't1']]])
        );

        $this->assertSame(
            ['type' => 'data-contextUsage', 'data' => ['usedTokens' => 100, 'maxTokens' => 1000, 'percent' => 10.0]],
            $mapper->map(['type' => 'context.usage', 'usedTokens' => 100, 'maxTokens' => 1000, 'percent' => 10.0])
        );

        $this->assertSame(
            ['type' => 'data-chatTitle', 'data' => ['title' => 'My chat']],
            $mapper->map(['type' => 'chat.title', 'title' => 'My chat'])
        );

        $this->assertSame(
            ['type' => 'data-maxStepsReached', 'data' => ['maxSteps' => 30, 'canContinue' => true]],
            $mapper->map(['type' => 'max_steps', 'maxSteps' => 30, 'canContinue' => true])
        );
    }

    public function test_maps_message_end_to_finish(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertSame(
            [
                'type' => 'finish',
                'finishReason' => 'tool-calls',
                'usage' => ['promptTokens' => 1, 'completionTokens' => 2, 'totalTokens' => 3],
            ],
            $mapper->map([
                'type' => 'message.end',
                'finishReason' => 'tool-calls',
                'usage' => ['promptTokens' => 1, 'completionTokens' => 2, 'totalTokens' => 3],
            ])
        );
    }

    public function test_maps_error(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertSame(
            ['type' => 'error', 'error' => ['code' => 'rate_limit', 'message' => 'Slow down']],
            $mapper->map(['type' => 'error', 'code' => 'rate_limit', 'message' => 'Slow down'])
        );
    }

    public function test_returns_null_for_unmapped_events(): void
    {
        $mapper = new VercelDataProtocolMapper;

        $this->assertNull($mapper->map(['type' => 'message.start']));
        $this->assertNull($mapper->map(['type' => 'unknown']));
    }
}
