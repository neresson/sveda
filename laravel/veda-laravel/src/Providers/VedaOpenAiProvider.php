<?php

namespace Veda\Laravel\Providers;

use Illuminate\Contracts\Events\Dispatcher;
use Laravel\Ai\Gateway\TextGenerationLoop;
use Laravel\Ai\Providers\OpenAiProvider;
use Veda\Laravel\Gateway\VedaOpenAiGateway;
use Veda\Laravel\Gateway\VedaTextGenerationLoop;

class VedaOpenAiProvider extends OpenAiProvider
{
    public function __construct(array $config, Dispatcher $events)
    {
        parent::__construct(new VedaOpenAiGateway($events), $config, $events);
    }

    public function textGenerationLoop(): TextGenerationLoop
    {
        return $this->textGenerationLoop ??= new VedaTextGenerationLoop($this->textGateway());
    }
}
