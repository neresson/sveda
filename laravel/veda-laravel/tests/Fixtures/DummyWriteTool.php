<?php

namespace Veda\Laravel\Tests\Fixtures;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Tools\VedaTool;

class DummyWriteTool extends VedaTool
{
    public function name(): string
    {
        return 'dummy_write';
    }

    public function description(): string
    {
        return 'Writes dummy data.';
    }

    public function mode(): ToolMode
    {
        return ToolMode::Write;
    }

    public function domain(): string
    {
        return 'testing';
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'value' => $schema->string()->required(),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        return ['success' => true, 'data' => ['written' => $arguments['value'] ?? null]];
    }
}
