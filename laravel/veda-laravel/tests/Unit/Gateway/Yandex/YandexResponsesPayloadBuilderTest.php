<?php

namespace Veda\Laravel\Tests\Unit\Gateway\Yandex;

use Veda\Laravel\Gateway\ProviderSession;
use Veda\Laravel\Gateway\Yandex\YandexAiConfig;
use Veda\Laravel\Gateway\Yandex\YandexResponsesPayloadBuilder;
use Veda\Laravel\Tests\TestCase;

class YandexResponsesPayloadBuilderTest extends TestCase
{
    public function test_builds_responses_payload_with_model_uri_and_multimodal_input(): void
    {
        $config = new YandexAiConfig([
            'key' => 'test-key',
            'folder_id' => 'b1g23nvs6j0g3d97p00g',
            'models' => ['text' => ['default' => 'qwen3.6-35b-a3b/latest']],
        ]);

        $session = new ProviderSession(
            'yandex',
            'qwen3.6-35b-a3b/latest',
            'test-key',
            $config->chatEndpoint(),
            false,
            'Api-Key',
        );

        $builder = new YandexResponsesPayloadBuilder($config);
        $payload = $builder->fromChatPayload($session, [
            'messages' => [
                ['role' => 'system', 'content' => 'System rules'],
                [
                    'role' => 'user',
                    'content' => [
                        ['type' => 'text', 'text' => 'Describe the screen'],
                        ['type' => 'image_url', 'image_url' => ['url' => 'data:image/jpeg;base64,abc', 'detail' => 'low']],
                    ],
                ],
            ],
            'max_completion_tokens' => 1000,
            'tools' => [
                [
                    'type' => 'function',
                    'function' => [
                        'name' => 'teacher_get_host_page_outline',
                        'description' => 'Get outline',
                        'parameters' => ['type' => 'object', 'properties' => []],
                    ],
                ],
            ],
            'tool_choice' => 'auto',
        ]);

        $this->assertSame('gpt://b1g23nvs6j0g3d97p00g/qwen3.6-35b-a3b/latest', $payload['model']);
        $this->assertSame('System rules', $payload['instructions']);
        $this->assertSame(1000, $payload['max_output_tokens']);
        $this->assertIsArray($payload['input']);
        $this->assertSame('input_image', $payload['input'][0]['content'][1]['type']);
        $this->assertCount(1, $payload['tools']);
        $this->assertSame('teacher_get_host_page_outline', $payload['tools'][0]['name']);
        $this->assertArrayNotHasKey('function', $payload['tools'][0]);
    }

    public function test_normalizes_chat_completions_tools_for_responses_api(): void
    {
        $config = new YandexAiConfig([
            'key' => 'k',
            'folder_id' => 'b1g23nvs6j0g3d97p00g',
            'models' => ['text' => ['default' => 'qwen3.6-35b-a3b/latest']],
        ]);

        $session = new ProviderSession('yandex', 'qwen3.6-35b-a3b/latest', 'k', 'https://example.com');
        $builder = new YandexResponsesPayloadBuilder($config);

        $payload = $builder->fromChatPayload($session, [
            'messages' => [['role' => 'user', 'content' => 'Hi']],
            'tools' => [
                [
                    'type' => 'function',
                    'function' => [
                        'name' => 'teacher_search_knowledge_base',
                        'description' => 'Search KB',
                        'parameters' => ['type' => 'object', 'properties' => []],
                    ],
                ],
            ],
            'tool_choice' => [
                'type' => 'function',
                'function' => ['name' => 'teacher_search_knowledge_base'],
            ],
        ]);

        $this->assertSame('teacher_search_knowledge_base', $payload['tools'][0]['name']);
        $this->assertSame('Search KB', $payload['tools'][0]['description']);
        $this->assertSame('teacher_search_knowledge_base', $payload['tool_choice']['name']);
        $this->assertArrayNotHasKey('function', $payload['tool_choice']);
    }

    public function test_model_uri_is_built_from_folder_id_in_default_model_uri(): void
    {
        $config = new YandexAiConfig([
            'models' => ['text' => ['default' => 'emb://b1gtestfolder/text-search-doc/latest']],
        ]);

        $this->assertSame(
            'gpt://b1gtestfolder/yandex-qwen',
            $config->modelUri('yandex-qwen')
        );
    }
}
