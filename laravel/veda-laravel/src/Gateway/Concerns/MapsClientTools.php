<?php

namespace Veda\Laravel\Gateway\Concerns;

use Veda\Laravel\Services\RequestContext;

trait MapsClientTools
{
    /**
     * @param  array<int, array<string, mixed>>  $mappedBackendTools
     * @return array<int, array<string, mixed>>
     */
    protected function mapClientTools(array $mappedBackendTools, string $format): array
    {
        $context = RequestContext::current();
        if ($context === null || $context->clientTools === []) {
            return [];
        }

        $taken = [];
        foreach ($mappedBackendTools as $tool) {
            $name = $format === 'chat'
                ? ($tool['function']['name'] ?? null)
                : ($tool['name'] ?? null);
            if (is_string($name) && $name !== '') {
                $taken[] = strtolower($name);
            }
        }

        $mapped = [];
        foreach ($context->clientTools as $clientTool) {
            $name = $clientTool['name'];
            if (in_array(strtolower($name), $taken, true)) {
                continue;
            }

            $parameters = $clientTool['parameters'];
            if (! isset($parameters['type'])) {
                $parameters = array_merge(['type' => 'object'], $parameters);
            }
            if (! isset($parameters['properties'])) {
                $parameters['properties'] = new \stdClass;
            }

            $mapped[] = $format === 'chat'
                ? [
                    'type' => 'function',
                    'function' => [
                        'name' => $name,
                        'description' => $clientTool['description'],
                        'parameters' => $parameters,
                    ],
                ]
                : [
                    'type' => 'function',
                    'name' => $name,
                    'description' => $clientTool['description'],
                    'parameters' => $parameters,
                ];
        }

        return $mapped;
    }
}
