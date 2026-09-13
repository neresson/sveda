<?php

namespace Veda\Laravel\Tools;

use Illuminate\Support\Str;

class ToolArgumentNormalizer
{
    /**
     * @param  array<string, mixed>  $arguments
     * @return array<string, mixed>
     */
    public function normalize(string $toolName, array $arguments): array
    {
        $arguments = $this->unwrapArguments($arguments);

        return $this->mapKeysToSnakeCase($arguments);
    }

    /**
     * @param  array<string, mixed>  $arguments
     * @return array<string, mixed>
     */
    protected function unwrapArguments(array $arguments): array
    {
        if ($arguments === []) {
            return $arguments;
        }

        foreach (['function', 'parameters', 'params', 'input', 'payload', 'data'] as $wrapper) {
            if (! array_key_exists($wrapper, $arguments)) {
                continue;
            }

            $nested = $arguments[$wrapper];
            unset($arguments[$wrapper]);

            if (is_string($nested)) {
                $decoded = json_decode($nested, true);
                if (is_array($decoded)) {
                    $arguments = array_merge($arguments, $decoded);
                }

                continue;
            }

            if (! is_array($nested)) {
                continue;
            }

            if (isset($nested['arguments']) && is_string($nested['arguments'])) {
                $decoded = json_decode($nested['arguments'], true);
                if (is_array($decoded)) {
                    $nested = array_merge($nested, $decoded);
                }
            }

            $arguments = array_merge($arguments, $nested);
        }

        return $arguments;
    }

    /**
     * @param  array<string, mixed>  $arguments
     * @return array<string, mixed>
     */
    protected function mapKeysToSnakeCase(array $arguments): array
    {
        $mapped = [];

        foreach ($arguments as $key => $value) {
            if (! is_string($key)) {
                $mapped[$key] = $value;

                continue;
            }

            $snakeKey = Str::snake(ltrim($key, '_'));
            if (! array_key_exists($snakeKey, $mapped)) {
                $mapped[$snakeKey] = $value;
            }
        }

        return $mapped;
    }
}
