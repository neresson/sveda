<?php

namespace Veda\Laravel\Tests\Unit\Gateway\Yandex;

use Veda\Laravel\Gateway\Yandex\YandexAiConfig;
use Veda\Laravel\Tests\TestCase;

class YandexAiConfigTest extends TestCase
{
    public function test_model_slug_returns_configured_default_when_model_missing(): void
    {
        $config = new YandexAiConfig([
            'models' => ['text' => ['default' => 'qwen3.6-35b-a3b/latest']],
        ]);

        $this->assertSame('qwen3.6-35b-a3b/latest', $config->modelSlug());
        $this->assertSame('qwen3.6-35b-a3b/latest', $config->modelSlug(''));
    }

    public function test_model_slug_strips_gpt_scheme_prefix(): void
    {
        $config = new YandexAiConfig([
            'folder_id' => 'b1gf',
            'models' => ['text' => ['default' => 'yandexgpt/latest']],
        ]);

        $this->assertSame('qwen3.6-35b-a3b/latest', $config->modelSlug('gpt://b1gf/qwen3.6-35b-a3b/latest'));
        $this->assertSame('yandex-qwen', $config->modelSlug('yandex-qwen'));
    }

    public function test_model_uri_prefixes_folder_and_passes_through_full_uris(): void
    {
        $config = new YandexAiConfig([
            'folder_id' => 'b1gf',
            'models' => ['text' => ['default' => 'yandexgpt/latest']],
        ]);

        $this->assertSame('gpt://b1gf/yandexgpt/latest', $config->modelUri('yandexgpt/latest'));
        $this->assertSame('gpt://b1gf/model/latest', $config->modelUri('gpt://other/model/latest'));
    }

    public function test_endpoints_fall_back_to_defaults(): void
    {
        $config = new YandexAiConfig([]);

        $this->assertSame(YandexAiConfig::DEFAULT_CHAT_ENDPOINT, $config->chatEndpoint());
        $this->assertSame(YandexAiConfig::DEFAULT_EMBEDDING_ENDPOINT, $config->embeddingEndpoint());
    }

    public function test_embedding_api_key_falls_back_to_main_key(): void
    {
        $config = new YandexAiConfig(['key' => 'main-key']);

        $this->assertSame('main-key', $config->embeddingApiKey());
        $this->assertSame('main-key', $config->apiKey());
    }

    public function test_folder_id_throws_when_not_configured(): void
    {
        $config = new YandexAiConfig([]);

        $this->expectException(\RuntimeException::class);

        $config->folderId();
    }
}
