<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Stringable;

class SearchCodeTool extends VedaTool
{
    use CodeIndexExecutor;

    public function name(): string
    {
        return 'search_code';
    }

    public function description(): Stringable|string
    {
        return 'Hybrid code search (semantic vectors + ripgrep keyword). Returns ranked code spans with snippets. Use read_code_index_file only when snippets are insufficient.';
    }

    public function domain(): string
    {
        return 'code_index';
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'query' => $schema->string()->description('Single search query. Use this OR queries.'),
            'queries' => $schema->array()->description('Multiple queries merged via RRF.')->items($schema->string()),
            'source_id' => $schema->integer()->description('Optional filter to one indexed source.'),
            'max_spans' => $schema->integer()->description('Max spans to return (1-40). Default 24.'),
            'limit' => $schema->integer()->description('Legacy alias for max_spans.'),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        return $this->searchCode($arguments);
    }
}
