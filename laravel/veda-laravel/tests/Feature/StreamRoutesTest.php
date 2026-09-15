<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Support\Facades\Queue;
use Laravel\Ai\Ai;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Models\VedaChatHistory;
use Veda\Laravel\Models\VedaGeneration;
use Veda\Laravel\Tests\Fixtures\CustomStreamController;
use Veda\Laravel\Tests\Fixtures\CustomVedaAgent;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\VedaManager;

class StreamRoutesTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.title_generation.enabled', false);
        $app['config']->set('veda.compaction.enabled', false);
        $app['config']->set('queue.default', 'sync');
    }

    public function test_stream_requires_authentication(): void
    {
        $this->postJson('/veda/stream', ['prompt' => 'Hi'])->assertStatus(401);
    }

    public function test_stream_returns_veda_protocol_events(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(VedaAgent::class, ['Hello from Veda']);

        $response = $this->actingAs($user)->postJson('/veda/stream', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-stream-1',
        ], [
            'Accept' => 'application/vnd.veda.stream+json',
        ]);

        $response->assertOk();
        $this->assertStringStartsWith('text/event-stream', (string) $response->headers->get('Content-Type'));

        $content = $response->streamedContent();

        $this->assertStringContainsString(': connected', $content);
        $this->assertStringContainsString('"type":"message.start"', $content);
        $this->assertStringContainsString('"type":"text.delta"', $content);
        $this->assertStringContainsString('"delta":"Hello"', $content);
        $this->assertStringContainsString('"delta":" Veda"', $content);
        $this->assertStringContainsString('"type":"message.end"', $content);
        $this->assertStringContainsString('data: [DONE]', $content);

        $generation = VedaGeneration::query()->first();
        $this->assertNotNull($generation);
        $this->assertSame(VedaGeneration::STATUS_COMPLETED, $generation->status);

        $history = VedaChatHistory::query()->where('chat_id', 'chat-stream-1')->first();
        $this->assertNotNull($history);
        $this->assertSame($user->getAuthIdentifier(), $history->user_id);
    }

    public function test_stream_extracts_prompt_when_messages_end_with_empty_assistant(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(VedaAgent::class, ['Hello from Veda']);

        $this->actingAs($user)->postJson('/veda/stream', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
                ['id' => 'm2', 'role' => 'assistant', 'parts' => []],
            ],
            'chatId' => 'chat-stream-placeholder',
        ], [
            'Accept' => 'application/vnd.veda.stream+json',
        ])->assertOk();

        $generation = VedaGeneration::query()->first();
        $this->assertNotNull($generation);
        $this->assertSame('Say hello', $generation->prompt);
    }

    public function test_stream_can_use_vercel_protocol(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(VedaAgent::class, ['Vercel response']);

        $response = $this->actingAs($user)->postJson('/veda/stream', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-stream-2',
        ], [
            'Accept' => 'text/event-stream',
            'X-Veda-Protocol' => 'vercel',
        ]);

        $response->assertOk();

        $content = $response->streamedContent();

        $this->assertStringContainsString('"type":"text-delta"', $content);
        $this->assertStringContainsString('"textDelta":"Vercel"', $content);
        $this->assertStringContainsString('"type":"data-contextUsage"', $content);
        $this->assertStringContainsString('"type":"finish"', $content);
        $this->assertStringContainsString('"finishReason":"stop"', $content);
        $this->assertStringContainsString('"usage"', $content);
        $this->assertStringContainsString('data: [DONE]', $content);
    }

    public function test_message_endpoint_returns_json(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(VedaAgent::class, ['Plain answer']);

        $response = $this->actingAs($user)->postJson('/veda/message', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-message-1',
        ]);

        $response->assertOk();
        $this->assertSame('Plain answer', $response->json('explanation'));
    }

    public function test_message_applies_registered_response_guard(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(VedaAgent::class, ['Plain answer']);

        app(VedaManager::class)->responseGuard(
            fn (string $text, array $context) => $text.' ['.$context['prompt'].']'
        );

        $response = $this->actingAs($user)->postJson('/veda/message', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-guard-1',
        ]);

        $response->assertOk();
        $this->assertSame('Plain answer [Say hello]', $response->json('explanation'));
    }

    public function test_controller_subclass_seams_are_used(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(CustomVedaAgent::class, ['Custom agent answer']);

        $this->app['router']->post('/custom/message', [CustomStreamController::class, 'message']);

        $response = $this->actingAs($user)->postJson('/custom/message', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-custom-1',
        ]);

        $response->assertOk();
        $this->assertSame('Custom agent answer', $response->json('explanation'));

        $history = VedaChatHistory::query()->where('chat_id', 'chat-custom-1')->first();
        $this->assertNotNull($history);
        $this->assertSame('custom-visitor', $history->visitor_id);
    }

    public function test_stream_accepts_camel_case_client_tools_and_context(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(VedaAgent::class, ['With tools']);

        $response = $this->actingAs($user)->postJson('/veda/stream', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Create a course'],
            ],
            'chatId' => 'chat-client-tools',
            'context' => [
                'page_type' => 'dashboard',
            ],
            'clientTools' => [
                [
                    'name' => 'confirm_course_creation',
                    'description' => 'Confirm',
                    'parameters' => ['type' => 'object', 'properties' => new \stdClass],
                ],
            ],
            'model' => 'deepseek-v4-flash',
            'options' => ['thinking' => true],
        ], [
            'Accept' => 'text/event-stream',
            'X-Veda-Protocol' => 'vercel',
        ]);

        $response->assertOk();
        $this->assertStringContainsString('"type":"text-delta"', $response->streamedContent());
    }

    public function test_stream_accepts_frontend_tool_result_continuation(): void
    {
        Queue::fake();

        $user = $this->createUser();

        Ai::fakeAgent(VedaAgent::class, ['Created']);

        $response = $this->actingAs($user)->postJson('/veda/stream', [
            'messages' => [
                ['id' => 'u1', 'role' => 'user', 'content' => 'Create a course'],
                [
                    'id' => 'a1',
                    'role' => 'assistant',
                    'parts' => [
                        [
                            'type' => 'tool-call',
                            'toolCallId' => 'tc1',
                            'toolName' => 'confirm_course_creation',
                            'input' => ['title' => 'Math'],
                        ],
                    ],
                ],
                [
                    'id' => 't1',
                    'role' => 'tool',
                    'parts' => [
                        [
                            'type' => 'tool-result',
                            'toolCallId' => 'tc1',
                            'toolName' => 'confirm_course_creation',
                            'output' => ['confirmed' => true],
                        ],
                    ],
                ],
            ],
            'chatId' => 'chat-tool-continue',
            'clientTools' => [
                [
                    'name' => 'confirm_course_creation',
                    'description' => 'Confirm',
                    'parameters' => ['type' => 'object'],
                ],
            ],
        ], [
            'Accept' => 'text/event-stream',
            'X-Veda-Protocol' => 'vercel',
        ]);

        $response->assertOk();
        $this->assertStringContainsString('"type":"text-delta"', $response->streamedContent());
    }
}
