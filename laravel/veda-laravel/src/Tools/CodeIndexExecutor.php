<?php

namespace Veda\Laravel\Tools;

use Veda\Laravel\CodeIndex\CodeIndexFileReaderService;
use Veda\Laravel\CodeIndex\CodeSearchRetrievalService;
use Veda\Laravel\Models\VedaCodeSource;

trait CodeIndexExecutor
{
    /**
     * @param  array<string, mixed>  $args
     * @return array<string, mixed>
     */
    protected function listCodeSources(array $args): array
    {
        $sources = VedaCodeSource::query()
            ->orderBy('name')
            ->get(['id', 'name', 'description', 'provider', 'status', 'last_indexed_at', 'metadata', 'error_message']);

        return [
            'success' => true,
            'data' => [
                'sources' => $sources->map(fn (VedaCodeSource $source) => [
                    'id' => $source->id,
                    'name' => $source->name,
                    'description' => $source->description,
                    'provider' => $source->provider,
                    'status' => $source->status,
                    'last_indexed_at' => $source->last_indexed_at?->toIso8601String(),
                    'chunks_total' => $source->metadata['chunks_total'] ?? null,
                    'files_indexed' => $source->metadata['files_indexed'] ?? null,
                    'search_index_ready' => (bool) ($source->metadata['search_index_ready'] ?? false),
                    'chunk_schema_version' => $source->metadata['chunk_schema_version'] ?? null,
                    'error_message' => $source->error_message,
                ])->values()->all(),
            ],
        ];
    }

    /**
     * @param  array<string, mixed>  $args
     * @return array<string, mixed>
     */
    protected function getCodeSourceOverview(array $args): array
    {
        $id = isset($args['source_id']) ? (int) $args['source_id'] : 0;
        if ($id <= 0) {
            return ['success' => false, 'error' => 'source_id is required'];
        }

        $source = VedaCodeSource::query()
            ->whereKey($id)
            ->first(['id', 'name', 'description', 'provider', 'status', 'structure_summary', 'metadata', 'last_indexed_at', 'error_message']);

        if (! $source) {
            return ['success' => false, 'error' => 'Code source not found'];
        }

        $summary = (string) ($source->structure_summary ?? '');
        $max = 12000;
        if (mb_strlen($summary, 'UTF-8') > $max) {
            $summary = mb_substr($summary, 0, $max, 'UTF-8').'…';
        }

        return [
            'success' => true,
            'data' => [
                'id' => $source->id,
                'name' => $source->name,
                'description' => $source->description,
                'provider' => $source->provider,
                'status' => $source->status,
                'last_indexed_at' => $source->last_indexed_at?->toIso8601String(),
                'metadata' => $source->metadata ?? [],
                'structure_summary' => $summary,
                'error_message' => $source->error_message,
            ],
        ];
    }

    /**
     * @param  array<string, mixed>  $args
     * @return array<string, mixed>
     */
    protected function searchCode(array $args): array
    {
        $queries = [];
        if (! empty($args['queries']) && is_array($args['queries'])) {
            $queries = array_values(array_filter(array_map('strval', $args['queries'])));
        } elseif (! empty($args['query']) && is_string($args['query'])) {
            $queries = [trim($args['query'])];
        }

        if ($queries === []) {
            return ['success' => false, 'error' => 'query or queries is required'];
        }

        $sourceId = isset($args['source_id']) ? (int) $args['source_id'] : null;
        $maxSpans = (int) config('veda.code_search.max_spans', 24);
        if (isset($args['max_spans'])) {
            $maxSpans = max(1, min(40, (int) $args['max_spans']));
        } elseif (isset($args['limit']) && $args['limit'] !== '') {
            $maxSpans = max(1, min(40, (int) $args['limit']));
        }

        return app(CodeSearchRetrievalService::class)->search(
            $queries,
            $sourceId > 0 ? $sourceId : null,
            $maxSpans,
        );
    }

    /**
     * @param  array<string, mixed>  $args
     * @return array<string, mixed>
     */
    protected function readCodeIndexFile(array $args): array
    {
        $reader = app(CodeIndexFileReaderService::class);
        $sourceId = isset($args['source_id']) ? (int) $args['source_id'] : 0;
        $pathRaw = isset($args['path']) ? trim((string) $args['path']) : '';
        $chunkId = isset($args['chunk_id']) ? (int) $args['chunk_id'] : 0;

        if ($pathRaw === '' && $chunkId > 0) {
            $resolved = $reader->resolvePathFromChunkId($chunkId);
            if ($resolved === null) {
                return ['success' => false, 'error' => 'Chunk not found'];
            }
            $sourceId = $resolved['source_id'];
            $pathRaw = $resolved['path'];
        }

        $maxLength = max(1, (int) config('veda.code_index.read_max_chars', 8000));
        $defaultLength = max(1, (int) config('veda.code_index.read_default_chars', 4000));
        $lengthArg = isset($args['length']) ? (int) $args['length'] : 0;
        $length = $lengthArg > 0 ? min($maxLength, max(1, $lengthArg)) : $defaultLength;

        $startLine = isset($args['start_line']) ? (int) $args['start_line'] : null;
        $endLine = isset($args['end_line']) ? (int) $args['end_line'] : null;

        return $reader->read(
            $sourceId,
            $pathRaw,
            max(0, (int) ($args['offset'] ?? 0)),
            $length,
            $startLine,
            $endLine,
        );
    }
}
