<?php

namespace Veda\Laravel\Providers;

use Illuminate\Contracts\Events\Dispatcher;
use Laravel\Ai\Contracts\Gateway\EmbeddingGateway;
use Laravel\Ai\Contracts\Gateway\StepTextGateway;
use Laravel\Ai\Contracts\Providers\EmbeddingProvider;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Gateway\TextGenerationLoop;
use Laravel\Ai\Providers\Concerns\GeneratesEmbeddings;
use Laravel\Ai\Providers\Concerns\GeneratesText;
use Laravel\Ai\Providers\Concerns\HasEmbeddingGateway;
use Laravel\Ai\Providers\Concerns\HasTextGateway;
use Laravel\Ai\Providers\Concerns\StreamsText;
use Laravel\Ai\Providers\Provider;
use Veda\Laravel\Gateway\VedaTextGenerationLoop;
use Veda\Laravel\Gateway\Yandex\YandexEmbeddingGateway;
use Veda\Laravel\Gateway\Yandex\YandexGateway;
use Veda\Laravel\Gateway\Yandex\YandexTextGateway;

class YandexTextProvider extends Provider implements EmbeddingProvider, TextProvider
{
    use GeneratesEmbeddings;
    use GeneratesText;
    use HasEmbeddingGateway;
    use HasTextGateway;
    use StreamsText;

    public function __construct(protected array $config, protected Dispatcher $events)
    {
        parent::__construct(
            gateway: new YandexGateway($events, $config),
            config: $config,
            events: $events
        );
    }

    public function textGateway(): StepTextGateway
    {
        return $this->textGateway ??= new YandexTextGateway($this->events, $this->config);
    }

    public function textGenerationLoop(): TextGenerationLoop
    {
        return $this->textGenerationLoop ??= new VedaTextGenerationLoop($this->textGateway());
    }

    public function embeddingGateway(): EmbeddingGateway
    {
        return $this->embeddingGateway ??= new YandexEmbeddingGateway($this->events, $this->config);
    }

    public function defaultEmbeddingsModel(): string
    {
        return $this->config['models']['embedding']['default'] ?? 'text-search-doc/latest';
    }

    public function defaultEmbeddingsDimensions(): int
    {
        return (int) ($this->config['models']['embedding']['dimensions'] ?? 256);
    }

    public function defaultTextModel(): string
    {
        return $this->config['models']['text']['default'] ?? 'yandexgpt/latest';
    }

    public function cheapestTextModel(): string
    {
        return $this->config['models']['text']['cheapest'] ?? 'yandexgpt-lite/latest';
    }

    public function smartestTextModel(): string
    {
        return $this->config['models']['text']['smartest'] ?? 'yandexgpt/latest';
    }
}
