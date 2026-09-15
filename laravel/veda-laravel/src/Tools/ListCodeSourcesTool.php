<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Stringable;

class ListCodeSourcesTool extends VedaTool
{
    use CodeIndexExecutor;

    public function name(): string
    {
        return 'list_code_sources';
    }

    public function description(): Stringable|string
    {
        return 'List connected code repositories or local folders indexed for this Veda instance. Use this list first to pick source_id before searching or reading files.';
    }

    public function domain(): string
    {
        return 'code_index';
    }

    public function schema(JsonSchema $schema): array
    {
        return [];
    }

    protected function execute(array $arguments): array|string
    {
        return $this->listCodeSources($arguments);
    }
}
