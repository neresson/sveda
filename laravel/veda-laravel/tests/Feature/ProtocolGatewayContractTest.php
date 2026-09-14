<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Http\Client\Request;
use Illuminate\Support\Facades\Http;
use Veda\Laravel\Services\VedaModelCatalog;
use Veda\Laravel\Tests\TestCase;

class ProtocolGatewayContractTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.deepseek.key', 'test-deepseek-key');
        $app['config']->set('veda.title_generation.enabled', false);
        $app['config']->set('veda.compaction.enabled', false);
        $app['config']->set('veda.tool_defer.enabled', false);
        $app['config']->set('veda.tool_catalog.embeddings_enabled', false);
        $app['config']->set('queue.default', 'sync');
    }

    public function test_responses_protocol_posts_to_deepseek_responses_endpoint(): void
    {
        Http::preventStrayRequests();
        Http::fake([
            'https://api.deepseek.com/responses' => Http::response($this->responsesPayload('Hello from responses')),
        ]);

        $user = $this->createUser();

        $this->actingAs($user)->postJson('/veda/message', [
            'prompt' => 'Say hello',
            'model' => 'deepseek-v4-flash',
            'options' => ['thinking' => true],
        ])->assertOk()->assertJsonPath('explanation', 'Hello from responses');

        Http::assertSent(function (Request $request): bool {
            $body = $request->data();

            return $request->url() === 'https://api.deepseek.com/responses'
                && ($body['model'] ?? null) === 'deepseek-v4-flash'
                && array_key_exists('input', $body)
                && ($body['reasoning']['effort'] ?? null) === 'high'
                && ($body['store'] ?? null) === false
                && ! in_array('reasoning.encrypted_content', $body['include'] ?? [], true);
        });
    }

    public function test_responses_thinking_off_sends_effort_none(): void
    {
        Http::preventStrayRequests();
        Http::fake([
            'https://api.deepseek.com/responses' => Http::response($this->responsesPayload('Fast')),
        ]);

        $user = $this->createUser();

        $this->actingAs($user)->postJson('/veda/message', [
            'prompt' => 'Say hello',
            'model' => 'deepseek-v4-flash-responses',
            'options' => ['thinking' => false],
        ])->assertOk();

        Http::assertSent(fn (Request $request): bool => ($request->data()['reasoning']['effort'] ?? null) === 'none');
    }

    public function test_anthropic_protocol_posts_to_deepseek_messages_endpoint(): void
    {
        Http::preventStrayRequests();
        Http::fake([
            'https://api.deepseek.com/anthropic/v1/messages' => Http::response($this->anthropicPayload('Hello from anthropic')),
        ]);

        $user = $this->createUser();

        $this->actingAs($user)->postJson('/veda/message', [
            'prompt' => 'Say hello',
            'model' => 'deepseek-v4-flash-anthropic',
            'options' => ['thinking' => true],
        ])->assertOk()->assertJsonPath('explanation', 'Hello from anthropic');

        Http::assertSent(function (Request $request): bool {
            $body = $request->data();

            return $request->url() === 'https://api.deepseek.com/anthropic/v1/messages'
                && ($body['model'] ?? null) === 'deepseek-v4-flash'
                && array_key_exists('messages', $body)
                && array_key_exists('system', $body)
                && ($body['thinking']['type'] ?? null) === 'enabled';
        });
    }

    public function test_anthropic_thinking_off_sends_disabled(): void
    {
        Http::preventStrayRequests();
        Http::fake([
            'https://api.deepseek.com/anthropic/v1/messages' => Http::response($this->anthropicPayload('Fast')),
        ]);

        $user = $this->createUser();

        $this->actingAs($user)->postJson('/veda/message', [
            'prompt' => 'Say hello',
            'model' => 'deepseek-v4-flash-anthropic',
            'options' => ['thinking' => false],
        ])->assertOk();

        Http::assertSent(fn (Request $request): bool => ($request->data()['thinking']['type'] ?? null) === 'disabled');
    }

    public function test_custom_catalog_model_uses_its_protocol_url(): void
    {
        $models = config('veda.models');
        $models[] = [
            'id' => 'custom-anthropic',
            'label' => 'Custom Anthropic',
            'protocol' => 'anthropic',
            'api_model' => 'custom-sonnet',
            'url' => 'https://llm.example.test/v1',
            'key' => 'custom-key',
            'thinking' => true,
            'vision' => false,
        ];
        config(['veda.models' => $models]);
        app(VedaModelCatalog::class)->registerIntoAi();

        Http::preventStrayRequests();
        Http::fake([
            'https://llm.example.test/v1/messages' => Http::response($this->anthropicPayload('Custom hello')),
        ]);

        $user = $this->createUser();

        $this->actingAs($user)->postJson('/veda/message', [
            'prompt' => 'Say hello',
            'model' => 'custom-anthropic',
        ])->assertOk()->assertJsonPath('explanation', 'Custom hello');

        Http::assertSent(fn (Request $request): bool => $request->url() === 'https://llm.example.test/v1/messages'
            && ($request->data()['model'] ?? null) === 'custom-sonnet');
    }

    public function test_unknown_model_returns_422(): void
    {
        $user = $this->createUser();

        $this->actingAs($user)->postJson('/veda/message', [
            'prompt' => 'Say hello',
            'model' => 'no-such-model',
        ])->assertStatus(422);
    }

    public function test_function_tools_use_responses_shape(): void
    {
        Http::preventStrayRequests();
        Http::fake([
            'https://api.deepseek.com/responses' => Http::response($this->responsesPayload('ok')),
        ]);

        $user = $this->createUser();

        $this->actingAs($user)->postJson('/veda/message', [
            'prompt' => 'Say hello',
            'model' => 'deepseek-v4-flash-responses',
            'clientTools' => [
                [
                    'name' => 'confirm_course_creation',
                    'description' => 'Confirm',
                    'parameters' => ['type' => 'object', 'properties' => new \stdClass],
                ],
            ],
        ])->assertOk();

        Http::assertSent(function (Request $request): bool {
            $tools = $request->data()['tools'] ?? [];
            foreach ($tools as $tool) {
                if (($tool['type'] ?? null) === 'function' && ($tool['name'] ?? null) === 'confirm_course_creation') {
                    return isset($tool['parameters']) && ! isset($tool['function']);
                }
            }

            return false;
        });
    }

    /**
     * @return array<string, mixed>
     */
    protected function responsesPayload(string $text): array
    {
        return [
            'id' => 'resp_test',
            'status' => 'completed',
            'model' => 'deepseek-v4-flash',
            'output' => [
                [
                    'type' => 'message',
                    'status' => 'completed',
                    'content' => [
                        ['type' => 'output_text', 'text' => $text],
                    ],
                ],
            ],
            'usage' => [
                'input_tokens' => 12,
                'output_tokens' => 8,
            ],
        ];
    }

    /**
     * @return array<string, mixed>
     */
    protected function anthropicPayload(string $text): array
    {
        return [
            'id' => 'msg_test',
            'type' => 'message',
            'role' => 'assistant',
            'model' => 'deepseek-v4-flash',
            'content' => [
                ['type' => 'text', 'text' => $text],
            ],
            'stop_reason' => 'end_turn',
            'usage' => [
                'input_tokens' => 12,
                'output_tokens' => 8,
            ],
        ];
    }
}
