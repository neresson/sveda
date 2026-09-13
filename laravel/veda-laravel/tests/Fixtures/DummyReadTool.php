<?php

namespace Veda\Laravel\Tests\Fixtures;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Tools\VedaTool;

class DummyReadTool extends VedaTool
{
    public function name(): string
    {
        return 'dummy_read';
    }

    public function description(): string
    {
        return 'Reads dummy data.';
    }

    public function mode(): ToolMode
    {
        return ToolMode::Read;
    }

    public function domain(): string
    {
        return 'testing';
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'query' => $schema->string()->required(),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        return ['success' => true, 'data' => ['query' => $arguments['query'] ?? null]];
    }
}
