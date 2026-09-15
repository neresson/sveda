<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Stringable;

class GetCodeSourceOverviewTool extends VedaTool
{
    use CodeIndexExecutor;

    public function name(): string
    {
        return 'get_code_source_overview';
    }

    public function description(): Stringable|string
    {
        return 'Return metadata and high-level project structure for one source. Includes the optional admin description field. Use after list_code_sources to understand layout before search_code.';
    }

    public function domain(): string
    {
        return 'code_index';
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'source_id' => $schema->integer()->description('ID from list_code_sources')->required(),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        return $this->getCodeSourceOverview($arguments);
    }
}
