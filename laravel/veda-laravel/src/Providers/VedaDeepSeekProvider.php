<?php

namespace Veda\Laravel\Providers;

use Laravel\Ai\Contracts\Gateway\StepTextGateway;
use Laravel\Ai\Gateway\TextGenerationLoop;
use Laravel\Ai\Providers\DeepSeekProvider;
use Veda\Laravel\Gateway\VedaDeepSeekGateway;
use Veda\Laravel\Gateway\VedaTextGenerationLoop;

class VedaDeepSeekProvider extends DeepSeekProvider
{
    public function textGateway(): StepTextGateway
    {
        return $this->textGateway ??= new VedaDeepSeekGateway($this->events);
    }

    public function textGenerationLoop(): TextGenerationLoop
    {
        return $this->textGenerationLoop ??= new VedaTextGenerationLoop($this->textGateway());
    }
}
