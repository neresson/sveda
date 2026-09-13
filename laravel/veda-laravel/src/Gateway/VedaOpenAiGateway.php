<?php

namespace Veda\Laravel\Gateway;

use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\OpenAi\OpenAiGateway;
use Laravel\Ai\Providers\Provider;
use Veda\Laravel\Gateway\Concerns\MapsActiveTools;
use Veda\Laravel\Gateway\Concerns\MapsClientTools;

class VedaOpenAiGateway extends OpenAiGateway
{
    use MapsActiveTools;
    use MapsClientTools;

    protected function mapTools(array $tools, Provider $provider): array
    {
        $mapped = [];
        $activeTools = $this->activeToolNames();

        foreach ($tools as $tool) {
            if ($tool instanceof Tool && $this->shouldMapTool($tool, $activeTools)) {
                $mapped[] = $this->mapTool($tool);
            }
        }

        return [...$mapped, ...$this->mapClientTools($mapped, 'responses')];
    }
}
