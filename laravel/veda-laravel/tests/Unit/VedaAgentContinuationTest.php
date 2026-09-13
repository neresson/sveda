<?php

namespace Veda\Laravel\Tests\Unit;

use Laravel\Ai\Messages\AssistantMessage;
use Laravel\Ai\Messages\ToolResultMessage;
use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\ToolResult;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Tests\TestCase;

class VedaAgentContinuationTest extends TestCase
{
    protected function tearDown(): void
    {
        RequestContext::forget();

        parent::tearDown();
    }

    public function test_messages_include_pending_frontend_tool_results(): void
    {
        RequestContext::bind(
            [],
            false,
            pendingToolCalls: [
                new ToolCall('tc1', 'confirm_course_creation', ['title' => 'Math']),
            ],
            pendingToolResults: [
                new ToolResult('tc1', 'confirm_course_creation', ['title' => 'Math'], ['confirmed' => true]),
            ],
        );

        $messages = collect((new VedaAgent)->messages());

        $this->assertInstanceOf(AssistantMessage::class, $messages->get(0));
        $this->assertInstanceOf(ToolResultMessage::class, $messages->get(1));
        $this->assertSame('confirm_course_creation', $messages->get(1)->toolResults->first()->name);
        $this->assertSame(['confirmed' => true], $messages->get(1)->toolResults->first()->result);
    }
}
