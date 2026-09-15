<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Stringable;

class ReadCodeIndexFileTool extends VedaTool
{
    use CodeIndexExecutor;

    public function name(): string
    {
        return 'read_code_index_file';
    }

    public function description(): Stringable|string
    {
        return 'Read file content from an indexed workspace by path. Use source_id and path from search_code, or pass chunk_id when path is omitted. Supports chunked reading for long files. For code navigation prefer start_line/end_line.';
    }

    public function domain(): string
    {
        return 'code_index';
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'source_id' => $schema->integer()->description('ID from list_code_sources / search_code')->required(),
            'path' => $schema->string()->description('Repository-relative path from search results. Optional if chunk_id is set.'),
            'chunk_id' => $schema->integer()->description('Optional chunk id from search_code; resolves path when path is omitted.'),
            'offset' => $schema->integer()->description('Character offset into merged file content. Ignored when start_line is set.'),
            'length' => $schema->integer()->description('Max characters to return (default 4000, max 8000).'),
            'start_line' => $schema->integer()->description('1-based start line for line-based reading.'),
            'end_line' => $schema->integer()->description('1-based inclusive end line. Defaults to start_line + 249 when omitted.'),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        return $this->readCodeIndexFile($arguments);
    }
}
