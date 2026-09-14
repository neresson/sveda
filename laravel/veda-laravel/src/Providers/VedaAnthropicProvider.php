<?php

namespace Veda\Laravel\Providers;

use Illuminate\Contracts\Events\Dispatcher;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Gateway\TextGenerationLoop;
use Laravel\Ai\Providers\Concerns\GeneratesText;
use Laravel\Ai\Providers\Concerns\HasTextGateway;
use Laravel\Ai\Providers\Concerns\StreamsText;
use Laravel\Ai\Providers\Provider;
use Veda\Laravel\Gateway\VedaAnthropicGateway;
use Veda\Laravel\Gateway\VedaTextGenerationLoop;

class VedaAnthropicProvider extends Provider implements TextProvider
{
    use GeneratesText;
    use HasTextGateway;
    use StreamsText;

    public function __construct(array $config, Dispatcher $events)
    {
        parent::__construct(new VedaAnthropicGateway($events), $config, $events);
    }

    public function textGenerationLoop(): TextGenerationLoop
    {
        return $this->textGenerationLoop ??= new VedaTextGenerationLoop($this->textGateway());
    }

    public function defaultTextModel(): string
    {
        return $this->config['models']['text']['default'] ?? 'deepseek-v4-flash';
    }

    public function cheapestTextModel(): string
    {
        return $this->config['models']['text']['cheapest'] ?? $this->defaultTextModel();
    }

    public function smartestTextModel(): string
    {
        return $this->config['models']['text']['smartest'] ?? $this->defaultTextModel();
    }
}
