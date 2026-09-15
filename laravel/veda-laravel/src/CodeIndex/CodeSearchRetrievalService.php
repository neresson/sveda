<?php

namespace Veda\Laravel\CodeIndex;

use Illuminate\Support\Collection;
use Illuminate\Support\Facades\DB;
use Veda\Laravel\Models\VedaCodeIndexChunk;
use Veda\Laravel\Models\VedaCodeSource;

class CodeSearchRetrievalService
{
    public function __construct(
        protected CodeIndexEmbeddingService $embeddingService,
        protected CodeIndexRipgrepService $ripgrep,
        protected CodeSymbolAwareChunkBuilder $chunkBuilder,
    ) {}

    /**
     * @param  array<int, string>  $queries
     * @return array<string, mixed>
     */
    public function search(array $queries, ?int $sourceId = null, int $maxSpans = 24): array
    {
        $queries = array_values(array_filter(array_map('trim', $queries)));
        if ($queries === []) {
            return ['success' => false, 'error' => 'query or queries is required'];
        }

        $maxSpans = max(1, min(40, $maxSpans));
        $maxPerFile = (int) config('veda.code_search.max_spans_per_file', 4);
        $excerptChars = (int) config('veda.code_search.excerpt_chars', 1000);
        $rrfK = (int) config('veda.code_search.rrf_k', 60);
        $weights = config('veda.code_search.weights', [
            'semantic' => 1.0,
            'keyword' => 0.8,
            'symbol' => 1.0,
        ]);

        $sources = $this->resolveSources($sourceId);
        if ($sources->isEmpty()) {
            return ['success' => false, 'error' => 'No searchable code sources found'];
        }

        $semanticHits = [];
        $keywordHits = [];

        foreach ($sources as $source) {
            $semanticHits = array_merge(
                $semanticHits,
                $this->semanticChannel($source, $queries, $maxSpans * 3)
            );

            $root = app(CodeSourceIndexer::class)->resolveWorkspaceRoot($source);
            if ($root !== null && is_dir($root) && $this->ripgrep->isAvailable()) {
                foreach ($this->ripgrep->search($source, $root, $queries) as $hit) {
                    $keywordHits[] = [
                        'source_id' => (int) $source->id,
                        'path' => $hit['path'],
                        'start_line' => $hit['line'],
                        'end_line' => $hit['end_line'],
                        'chunk_id' => null,
                        'chunk_kind' => CodeSymbolAwareChunkBuilder::KIND_LINES,
                        'symbol' => null,
                        'content' => $hit['text'],
                        'channel' => 'keyword',
                    ];
                }
            } else {
                $keywordHits = array_merge(
                    $keywordHits,
                    $this->sqlKeywordChannel($source, $queries, $maxSpans * 2)
                );
            }
        }

        $fused = $this->fuseRrf(
            [
                'semantic' => $semanticHits,
                'keyword' => $keywordHits,
            ],
            $weights,
            $rrfK,
            $maxSpans,
            $maxPerFile,
        );

        $enriched = $this->enrichSpans($fused, $excerptChars);

        return [
            'success' => true,
            'data' => [
                'queries' => $queries,
                'spans' => $enriched,
                'span_count' => count($enriched),
                'search_method' => 'hybrid_rrf',
                'index_coverage' => $this->indexCoverage($sources),
                'notice' => 'Use read_code_index_file only when snippets are insufficient.',
            ],
        ];
    }

    /**
     * @param  array<int, string>  $queries
     * @return list<array<string, mixed>>
     */
    protected function semanticChannel(VedaCodeSource $source, array $queries, int $limit): array
    {
        if (! $this->embeddingService->isAvailable()) {
            return [];
        }

        $hits = [];
        foreach ($queries as $query) {
            $embedding = $this->embeddingService->createQueryEmbedding($query, 3);
            if (! $embedding) {
                continue;
            }

            $rows = VedaCodeIndexChunk::query()
                ->where('code_source_id', $source->id)
                ->whereNotNull('embedding')
                ->where('chunk_kind', CodeSymbolAwareChunkBuilder::KIND_LINES)
                ->orderBy('id')
                ->get();

            $scored = [];
            foreach ($rows as $row) {
                $stored = $this->decodeEmbedding($row->embedding);
                if ($stored === null) {
                    continue;
                }
                $scored[] = [
                    'row' => $row,
                    'distance' => $this->cosineDistance($embedding, $stored),
                ];
            }

            usort($scored, fn (array $a, array $b): int => $a['distance'] <=> $b['distance']);
            foreach (array_slice($scored, 0, $limit) as $item) {
                $row = $item['row'];
                $hits[] = [
                    'source_id' => (int) $row->code_source_id,
                    'path' => (string) $row->path,
                    'start_line' => (int) $row->start_line,
                    'end_line' => (int) $row->end_line,
                    'chunk_id' => (int) $row->id,
                    'chunk_kind' => (string) ($row->chunk_kind ?? CodeSymbolAwareChunkBuilder::KIND_LINES),
                    'symbol' => $row->qualified_name ?? $row->symbol_name,
                    'content' => (string) $row->content,
                    'distance' => (float) $item['distance'],
                    'channel' => 'semantic',
                ];
            }
        }

        return $hits;
    }

    /**
     * @param  array<int, string>  $queries
     * @return list<array<string, mixed>>
     */
    protected function sqlKeywordChannel(VedaCodeSource $source, array $queries, int $limit): array
    {
        $hits = [];
        foreach ($queries as $query) {
            $rows = VedaCodeIndexChunk::query()
                ->where('code_source_id', $source->id)
                ->whereRaw('LOWER(content) LIKE ?', ['%'.mb_strtolower($query, 'UTF-8').'%'])
                ->limit($limit)
                ->get();

            foreach ($rows as $row) {
                $hits[] = [
                    'source_id' => (int) $row->code_source_id,
                    'path' => (string) $row->path,
                    'start_line' => (int) $row->start_line,
                    'end_line' => (int) $row->end_line,
                    'chunk_id' => (int) $row->id,
                    'chunk_kind' => (string) ($row->chunk_kind ?? CodeSymbolAwareChunkBuilder::KIND_LINES),
                    'symbol' => $row->qualified_name ?? $row->symbol_name,
                    'content' => (string) $row->content,
                    'channel' => 'keyword',
                ];
            }
        }

        return $hits;
    }

    /**
     * @param  array<string, list<array<string, mixed>>>  $channelHits
     * @param  array<string, float>  $weights
     * @return list<array<string, mixed>>
     */
    protected function fuseRrf(
        array $channelHits,
        array $weights,
        int $rrfK,
        int $maxSpans,
        int $maxPerFile,
    ): array {
        $scores = [];
        foreach ($channelHits as $channel => $hits) {
            $weight = (float) ($weights[$channel] ?? 1.0);
            usort($hits, function (array $a, array $b): int {
                $da = $a['distance'] ?? 999.0;
                $db = $b['distance'] ?? 999.0;

                return $da <=> $db;
            });
            $rank = 0;
            foreach ($hits as $hit) {
                $rank++;
                $key = $hit['source_id']."\0".$hit['path']."\0".$hit['start_line']."\0".$hit['end_line'];
                if (! isset($scores[$key])) {
                    $scores[$key] = [
                        'hit' => $hit,
                        'score' => 0.0,
                        'channels' => [],
                    ];
                }
                $scores[$key]['score'] += $weight * (1.0 / ($rrfK + $rank));
                if (! in_array($channel, $scores[$key]['channels'], true)) {
                    $scores[$key]['channels'][] = $channel;
                }
            }
        }

        $ranked = array_values($scores);
        usort($ranked, fn (array $a, array $b) => $b['score'] <=> $a['score']);

        $result = [];
        $perFile = [];
        foreach ($ranked as $entry) {
            $hit = $entry['hit'];
            $fileKey = $hit['source_id']."\0".$hit['path'];
            $perFile[$fileKey] = ($perFile[$fileKey] ?? 0) + 1;
            if ($perFile[$fileKey] > $maxPerFile) {
                continue;
            }
            $hit['score'] = round($entry['score'], 4);
            $hit['channels'] = $entry['channels'];
            $result[] = $hit;
            if (count($result) >= $maxSpans) {
                break;
            }
        }

        return $result;
    }

    /**
     * @param  list<array<string, mixed>>  $spans
     * @return list<array<string, mixed>>
     */
    protected function enrichSpans(array $spans, int $excerptChars): array
    {
        $indexer = app(CodeSourceIndexer::class);
        $enriched = [];

        foreach ($spans as $span) {
            $source = VedaCodeSource::query()->find($span['source_id']);
            $snippet = mb_substr((string) ($span['content'] ?? ''), 0, $excerptChars, 'UTF-8');
            $excerptFrom = 'index';

            if ($source) {
                $root = $indexer->resolveWorkspaceRoot($source);
                if ($root !== null) {
                    $disk = $this->readLinesFromDisk(
                        $root,
                        (string) $span['path'],
                        (int) $span['start_line'],
                        (int) $span['end_line'],
                        $excerptChars,
                    );
                    if ($disk !== '') {
                        $snippet = $disk;
                        $excerptFrom = 'workspace';
                    }
                }
            }

            $enriched[] = [
                'source_id' => $span['source_id'],
                'path' => $span['path'],
                'start_line' => $span['start_line'],
                'end_line' => $span['end_line'],
                'chunk_id' => $span['chunk_id'] ?? null,
                'chunk_kind' => $span['chunk_kind'] ?? CodeSymbolAwareChunkBuilder::KIND_LINES,
                'symbol' => $span['symbol'] ?? null,
                'score' => $span['score'] ?? 0.0,
                'channels' => $span['channels'] ?? [],
                'snippet' => $snippet,
                'excerpt_from' => $excerptFrom,
            ];
        }

        return $enriched;
    }

    protected function readLinesFromDisk(string $root, string $path, int $startLine, int $endLine, int $maxChars): string
    {
        $absolute = $root.DIRECTORY_SEPARATOR.str_replace('/', DIRECTORY_SEPARATOR, ltrim($path, '/'));
        if (! is_file($absolute)) {
            return '';
        }

        $lines = @file($absolute, FILE_IGNORE_NEW_LINES);
        if (! is_array($lines)) {
            return '';
        }

        $slice = array_slice($lines, max(0, $startLine - 1), max(1, $endLine - $startLine + 1));
        $text = implode("\n", $slice);

        return mb_substr($text, 0, $maxChars, 'UTF-8');
    }

    /**
     * @return Collection<int, VedaCodeSource>
     */
    protected function resolveSources(?int $sourceId): Collection
    {
        $query = VedaCodeSource::query()->where('status', 'ready');

        if ($sourceId !== null && $sourceId > 0) {
            $query->where('id', $sourceId);
        }

        return $query->get();
    }

    /**
     * @param  Collection<int, VedaCodeSource>  $sources
     * @return array<string, mixed>
     */
    protected function indexCoverage(Collection $sources): array
    {
        $chunks = 0;
        $ready = true;

        foreach ($sources as $source) {
            $meta = is_array($source->metadata) ? $source->metadata : [];
            if (! ($meta['search_index_ready'] ?? true)) {
                $ready = false;
            }
            $chunks += (int) ($meta['chunks_total'] ?? 0);
        }

        return [
            'chunk_schema_version' => CodeSymbolAwareChunkBuilder::CHUNK_SCHEMA_VERSION,
            'chunks_indexed' => $chunks,
            'search_index_ready' => $ready,
            'ripgrep_available' => $this->ripgrep->isAvailable(),
        ];
    }

    /**
     * @return list<float>|null
     */
    protected function decodeEmbedding(mixed $stored): ?array
    {
        if (is_array($stored)) {
            return array_map(static fn ($v): float => (float) $v, $stored);
        }
        if (! is_string($stored) || trim($stored) === '') {
            return null;
        }
        $decoded = json_decode($stored, true);
        if (! is_array($decoded) || $decoded === []) {
            return null;
        }

        return array_map(static fn ($v): float => (float) $v, $decoded);
    }

    /**
     * @param  list<float>  $a
     * @param  list<float>  $b
     */
    protected function cosineDistance(array $a, array $b): float
    {
        $n = min(count($a), count($b));
        if ($n === 0) {
            return 1.0;
        }
        $dot = 0.0;
        $na = 0.0;
        $nb = 0.0;
        for ($i = 0; $i < $n; $i++) {
            $dot += $a[$i] * $b[$i];
            $na += $a[$i] * $a[$i];
            $nb += $b[$i] * $b[$i];
        }
        $denom = sqrt($na) * sqrt($nb);
        if ($denom <= 0.0) {
            return 1.0;
        }

        return 1.0 - ($dot / $denom);
    }
}
