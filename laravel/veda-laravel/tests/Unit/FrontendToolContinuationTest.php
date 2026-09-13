<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Support\FrontendToolContinuation;
use Veda\Laravel\Tests\TestCase;

class FrontendToolContinuationTest extends TestCase
{
    public function test_returns_null_when_last_message_is_user(): void
    {
        $this->assertNull(FrontendToolContinuation::fromMessages([
            ['role' => 'user', 'content' => 'Hello'],
        ]));
    }

    public function test_returns_null_when_tool_message_has_no_result_parts(): void
    {
        $this->assertNull(FrontendToolContinuation::fromMessages([
            ['role' => 'assistant', 'content' => ''],
            ['role' => 'tool', 'parts' => [['type' => 'text', 'text' => 'nope']]],
        ]));
    }

    public function test_extracts_tool_calls_and_results(): void
    {
        $messages = [
            ['id' => 'm1', 'role' => 'user', 'content' => 'Create a course'],
            [
                'id' => 'm2',
                'role' => 'assistant',
                'content' => '',
                'parts' => [
                    [
                        'type' => 'tool-call',
                        'toolCallId' => 'call-1',
                        'toolName' => 'confirm_course_creation',
                        'target' => 'frontend',
                        'input' => ['title' => 'Math'],
                    ],
                ],
            ],
            [
                'id' => 'm3',
                'role' => 'tool',
                'parts' => [
                    [
                        'type' => 'tool-result',
                        'toolCallId' => 'call-1',
                        'toolName' => 'confirm_course_creation',
                        'output' => ['confirmed' => true],
                    ],
                ],
            ],
        ];

        $continuation = FrontendToolContinuation::fromMessages($messages);

        $this->assertNotNull($continuation);
        $this->assertCount(1, $continuation['toolCalls']);
        $this->assertCount(1, $continuation['toolResults']);

        $call = $continuation['toolCalls'][0];
        $this->assertSame('call-1', $call->id);
        $this->assertSame('call-1', $call->resultId);
        $this->assertSame('confirm_course_creation', $call->name);
        $this->assertSame(['title' => 'Math'], $call->arguments);

        $result = $continuation['toolResults'][0];
        $this->assertSame('call-1', $result->id);
        $this->assertSame('call-1', $result->resultId);
        $this->assertSame('confirm_course_creation', $result->name);
        $this->assertSame(['title' => 'Math'], $result->arguments);
        $this->assertSame(['confirmed' => true], $result->result);
    }

    public function test_result_without_matching_call_falls_back_to_result_name(): void
    {
        $continuation = FrontendToolContinuation::fromMessages([
            ['role' => 'user', 'content' => 'Hi'],
            [
                'role' => 'tool',
                'parts' => [
                    ['type' => 'tool-result', 'toolCallId' => 'call-9', 'toolName' => 'confirm_x', 'output' => ['ok' => true]],
                ],
            ],
        ]);

        $this->assertNotNull($continuation);
        $this->assertSame('confirm_x', $continuation['toolCalls'][0]->name);
        $this->assertSame([], $continuation['toolCalls'][0]->arguments);
    }

    public function test_multiple_results_in_one_tool_message(): void
    {
        $continuation = FrontendToolContinuation::fromMessages([
            [
                'role' => 'assistant',
                'parts' => [
                    ['type' => 'tool-call', 'toolCallId' => 'a', 'toolName' => 'tool_a', 'input' => ['x' => 1]],
                    ['type' => 'tool-call', 'toolCallId' => 'b', 'toolName' => 'tool_b', 'input' => ['y' => 2]],
                ],
            ],
            [
                'role' => 'tool',
                'parts' => [
                    ['type' => 'tool-result', 'toolCallId' => 'a', 'toolName' => 'tool_a', 'output' => 'ra'],
                    ['type' => 'tool-result', 'toolCallId' => 'b', 'toolName' => 'tool_b', 'output' => 'rb'],
                ],
            ],
        ]);

        $this->assertNotNull($continuation);
        $this->assertCount(2, $continuation['toolCalls']);
        $this->assertCount(2, $continuation['toolResults']);
        $this->assertSame(['x' => 1], $continuation['toolCalls'][0]->arguments);
        $this->assertSame('rb', $continuation['toolResults'][1]->result);
    }
}
