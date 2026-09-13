<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\ToolArgumentNormalizer;

class ToolArgumentNormalizerTest extends TestCase
{
    public function test_passes_plain_arguments_through(): void
    {
        $normalizer = new ToolArgumentNormalizer;

        $result = $normalizer->normalize('get_weather', ['city' => 'Berlin', 'units' => 'metric']);

        $this->assertSame(['city' => 'Berlin', 'units' => 'metric'], $result);
    }

    public function test_unwraps_function_wrapper(): void
    {
        $normalizer = new ToolArgumentNormalizer;

        $result = $normalizer->normalize('get_weather', ['function' => ['city' => 'Berlin']]);

        $this->assertSame(['city' => 'Berlin'], $result);
    }

    public function test_unwraps_parameters_wrapper(): void
    {
        $normalizer = new ToolArgumentNormalizer;

        $result = $normalizer->normalize('get_weather', ['parameters' => ['city' => 'Berlin']]);

        $this->assertSame(['city' => 'Berlin'], $result);
    }

    public function test_normalizes_keys_to_snake_case(): void
    {
        $normalizer = new ToolArgumentNormalizer;

        $result = $normalizer->normalize('get_weather', ['cityName' => 'Berlin', 'unitSystem' => 'metric']);

        $this->assertSame(['city_name' => 'Berlin', 'unit_system' => 'metric'], $result);
    }
}
