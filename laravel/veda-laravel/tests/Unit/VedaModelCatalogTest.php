<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Exceptions\UnknownVedaModelException;
use Veda\Laravel\Services\VedaModelCatalog;
use Veda\Laravel\Tests\TestCase;

class VedaModelCatalogTest extends TestCase
{
    public function test_resolves_alias_to_responses_preset(): void
    {
        $catalog = app(VedaModelCatalog::class);
        $model = $catalog->resolve('deepseek-v4-flash');

        $this->assertSame('deepseek-v4-flash-responses', $model->id);
        $this->assertSame('veda-model:deepseek-v4-flash-responses', $model->laravelProviderName());
        $this->assertSame('veda-responses', $model->protocol->driver());
        $this->assertSame('deepseek-v4-flash', $model->apiModel);
    }

    public function test_resolves_anthropic_preset_without_alias_collision(): void
    {
        $model = app(VedaModelCatalog::class)->resolve('deepseek-v4-flash-anthropic');

        $this->assertSame('deepseek-v4-flash-anthropic', $model->id);
        $this->assertSame('veda-anthropic', $model->protocol->driver());
        $this->assertSame('https://api.deepseek.com/anthropic/v1', $model->url);
    }

    public function test_registers_prefixed_laravel_ai_providers(): void
    {
        $catalog = app(VedaModelCatalog::class);
        $catalog->registerIntoAi();

        $this->assertSame('veda-responses', config('ai.providers.veda-model:deepseek-v4-flash-responses.driver'));
        $this->assertSame('veda-anthropic', config('ai.providers.veda-model:deepseek-v4-flash-anthropic.driver'));
        $this->assertSame('https://api.deepseek.com', config('ai.providers.veda-model:deepseek-v4-flash-responses.url'));
        $this->assertFalse(config('ai.providers.veda-model:deepseek-v4-flash-responses.store'));
    }

    public function test_stream_providers_skip_unknown_failover_ids(): void
    {
        config(['veda.failover' => ['missing-model', 'deepseek-v4-flash-anthropic']]);

        $catalog = app(VedaModelCatalog::class);
        $primary = $catalog->resolve('deepseek-v4-flash-responses');
        $providers = $catalog->streamProviders($primary);

        $this->assertSame([
            'veda-model:deepseek-v4-flash-responses' => 'deepseek-v4-flash',
            'veda-model:deepseek-v4-flash-anthropic' => 'deepseek-v4-flash',
        ], $providers);
    }

    public function test_unknown_model_throws(): void
    {
        $this->expectException(UnknownVedaModelException::class);

        app(VedaModelCatalog::class)->resolve('does-not-exist');
    }
}
