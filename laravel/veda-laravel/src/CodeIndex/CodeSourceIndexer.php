<?php

namespace Veda\Laravel\CodeIndex;

use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use Symfony\Component\Finder\Gitignore;
use Symfony\Component\Process\Process;
use Veda\Laravel\Models\VedaCodeIndexChunk;
use Veda\Laravel\Models\VedaCodeSource;
use Veda\Laravel\Support\Utf8Text;

class CodeSourceIndexer
{
    private const SKIP_DIR_NAMES = [
        'node_modules', 'vendor', '.git', 'dist', 'build', '.next', 'coverage',
        'bootstrap/cache', '.idea', '.vscode', '__pycache__', 'venv',
    ];

    private const EXTENSIONS = [
        'php', 'vue', 'js', 'jsx', 'mjs', 'cjs', 'ts', 'tsx', 'css', 'scss', 'sass',
        'less', 'md', 'mdx', 'json', 'yaml', 'yml', 'xml', 'html', 'blade.php',
        'sql', 'sh', 'env.example', 'ini', 'toml', 'rs', 'go', 'java', 'kt', 'swift',
    ];

    public function index(VedaCodeSource $source): void
    {
        $source->refresh();
        if ($source->status === 'cancelled') {
            return;
        }
        if ($source->index_cancel_requested) {
            $this->finishCancelled($source);

            return;
        }

        $excludePaths = $this->userExcludePaths($source);
        $storedHashesBeforeStart = $this->getStoredCodeIndexFileHashes($source);

        $updated = VedaCodeSource::query()
            ->whereKey($source->id)
            ->whereNotIn('status', ['cancelled'])
            ->update([
                'status' => 'indexing',
                'error_message' => null,
                'index_cancel_requested' => false,
                'indexing_progress' => 0,
                'indexing_phase' => 'starting',
                'metadata' => null,
                'updated_at' => now(),
            ]);

        if ($updated === 0) {
            return;
        }

        $source->refresh();

        try {
            $this->throwIfCancelled($source);
            $root = $this->resolveWorkspaceRoot($source);
            if ($root === null || ! is_dir($root)) {
                throw new \RuntimeException('workspace_not_resolved');
            }

            $this->reportProgress($source, 10, 'scan_files');
            $files = $this->collectFiles($root, $source, $excludePaths);
            $maxChunks = config('veda.code_index.max_total_chunks', 8000);
            $chunkMax = config('veda.code_index.chunk_max_chars', 1600);
            $chunkBuilder = app(CodeSymbolAwareChunkBuilder::class);

            $dbPaths = $this->distinctIndexedPaths($source->id);
            $useIncremental = false;
            $budgetForcedFull = false;
            $pathsToRefresh = [];
            $newHashes = [];

            if ($storedHashesBeforeStart !== null) {
                $newHashes = $this->computeIndexableFileContentHashes($files);
                $pathsToRefresh = $this->computePathsToRefresh($storedHashesBeforeStart, $newHashes, $dbPaths);
                if ($pathsToRefresh === []) {
                    $this->finalizeUnchangedIndex($source, $newHashes, count($files), $excludePaths);

                    return;
                }
                $currentChunkTotal = (int) VedaCodeIndexChunk::query()
                    ->where('code_source_id', $source->id)
                    ->count();
                $removing = $this->countChunksForSourcePaths($source->id, $pathsToRefresh);
                $afterRemoval = $currentChunkTotal - $removing;
                $incoming = $this->countChunksForPathsInFiles($files, $pathsToRefresh, $chunkMax);
                if ($afterRemoval + $incoming <= $maxChunks) {
                    $useIncremental = true;
                } else {
                    $budgetForcedFull = true;
                }
            }

            $inserted = 0;
            $batch = [];
            $fileList = [];

            if ($useIncremental) {
                $reuseEmbeddings = $this->collectReuseEmbeddingsMapForPaths($source->id, $pathsToRefresh);
                if ($pathsToRefresh !== []) {
                    foreach (array_chunk($pathsToRefresh, 400) as $pathChunk) {
                        VedaCodeIndexChunk::query()
                            ->where('code_source_id', $source->id)
                            ->whereIn('path', $pathChunk)
                            ->delete();
                    }
                }
                $baseAfterDelete = (int) VedaCodeIndexChunk::query()
                    ->where('code_source_id', $source->id)
                    ->count();
                $newChunkRoom = max(0, $maxChunks - $baseAfterDelete);
                $allowed = array_fill_keys($pathsToRefresh, true);
                $fileTotal = count($pathsToRefresh);
                $fileIndex = 0;
                foreach ($files as $relPath => $absPath) {
                    $this->throwIfCancelled($source);
                    if (! isset($allowed[$relPath])) {
                        continue;
                    }
                    if ($inserted >= $newChunkRoom) {
                        break;
                    }
                    if ($this->shouldNeverIndexFile($relPath)) {
                        continue;
                    }
                    $fileIndex++;
                    $insertedForFile = $this->appendChunksForFile(
                        $source,
                        $relPath,
                        $absPath,
                        $chunkMax,
                        $reuseEmbeddings,
                        $newChunkRoom,
                        $inserted,
                        $batch
                    );
                    $inserted += $insertedForFile;
                    if ($fileTotal > 0 && $fileIndex % 4 === 0) {
                        $pct = 35 + (int) (($fileIndex / max(1, $fileTotal)) * 38);
                        $this->reportProgress($source, min(72, $pct), 'build_chunks');
                    }
                }
                if ($batch !== []) {
                    DB::table((new VedaCodeIndexChunk)->getTable())->insert($batch);
                }
                $fileList = VedaCodeIndexChunk::query()
                    ->where('code_source_id', $source->id)
                    ->distinct()
                    ->orderBy('path')
                    ->pluck('path')
                    ->all();
            } else {
                $contentHashesByPath = [];
                $reuseEmbeddings = $this->collectReuseEmbeddingsMap($source->id);
                VedaCodeIndexChunk::where('code_source_id', $source->id)->delete();

                $fileTotal = count($files);
                $fileIndex = 0;
                foreach ($files as $relPath => $absPath) {
                    $this->throwIfCancelled($source);
                    if ($inserted >= $maxChunks) {
                        break;
                    }
                    if ($this->shouldNeverIndexFile($relPath)) {
                        continue;
                    }
                    $fileList[] = $relPath;
                    $content = @file_get_contents($absPath);
                    if ($content === false || $content === '') {
                        $fileIndex++;

                        continue;
                    }
                    $content = Utf8Text::sanitize($content, (int) config('veda.code_index.max_file_bytes', 524288) + 64);
                    if ($content === '') {
                        $fileIndex++;

                        continue;
                    }
                    if (! $this->isMostlyText($content)) {
                        $fileIndex++;

                        continue;
                    }
                    $contentHashesByPath[Utf8Text::sanitize($relPath, 2048)] = hash('sha256', $content);
                    foreach ($chunkBuilder->lineChunks($content, $chunkMax) as $idx => $chunk) {
                        $this->throwIfCancelled($source);
                        if ($inserted >= $maxChunks) {
                            break 2;
                        }
                        $sanPath = Utf8Text::sanitize($relPath, 2048);
                        $sanContent = Utf8Text::sanitize($chunk['text'], 4_194_304);
                        $fp = $chunkBuilder->contentFingerprint($sanPath, $chunk['start'], $chunk['end'], $sanContent);
                        $row = [
                            'code_source_id' => $source->id,
                            'path' => $sanPath,
                            'chunk_kind' => CodeSymbolAwareChunkBuilder::KIND_LINES,
                            'chunk_index' => $idx,
                            'start_line' => $chunk['start'],
                            'end_line' => $chunk['end'],
                            'content' => $sanContent,
                            'content_hash' => $fp,
                            'embedding' => $reuseEmbeddings[$fp] ?? null,
                            'created_at' => now(),
                            'updated_at' => now(),
                        ];
                        $batch[] = $row;
                        $inserted++;
                        if (count($batch) >= 200) {
                            DB::table((new VedaCodeIndexChunk)->getTable())->insert($batch);
                            $batch = [];
                        }
                    }
                    $fileIndex++;
                    if ($fileTotal > 0 && $fileIndex % 8 === 0) {
                        $pct = 35 + (int) (($fileIndex / $fileTotal) * 38);
                        $this->reportProgress($source, min(72, $pct), 'build_chunks');
                    }
                }
                if ($batch !== []) {
                    DB::table((new VedaCodeIndexChunk)->getTable())->insert($batch);
                }
                ksort($contentHashesByPath);
                $newHashes = $contentHashesByPath;
            }

            $chunksTotal = (int) VedaCodeIndexChunk::query()
                ->where('code_source_id', $source->id)
                ->count();

            $treeLines = array_slice($fileList, 0, 800);
            $safeLines = array_map(
                fn (string $p): string => Utf8Text::sanitize($p, 4096),
                $treeLines
            );
            $structure = 'Indexed files ('.count($fileList)."):\n".implode("\n", $safeLines);
            $structure = Utf8Text::sanitize($structure, 2_097_152);
            if (count($fileList) > 800) {
                $structure .= "\n…";
            }

            $embeddedAfterInsert = (int) DB::table((new VedaCodeIndexChunk)->getTable())
                ->where('code_source_id', $source->id)
                ->whereNotNull('embedding')
                ->count();

            $source->refresh();
            $meta = $this->mergeCodeGraphMetadata($source, [
                'files_indexed' => count($newHashes),
                'chunks_total' => $chunksTotal,
                'chunks_embedded' => $embeddedAfterInsert,
                'code_index_file_hashes' => $newHashes,
                'exclude_paths' => $excludePaths,
                'workspace_ready' => true,
            ]);

            $source->update([
                'structure_summary' => $structure,
                'metadata' => $meta,
                'status' => 'indexing',
            ]);

            $this->embedChunks($source->fresh());

            $this->throwIfCancelled($source->fresh());
            $fresh = $source->fresh();
            $meta = is_array($fresh?->metadata) ? $fresh->metadata : [];
            $meta['chunks_embedded'] = (int) DB::table((new VedaCodeIndexChunk)->getTable())
                ->where('code_source_id', $source->id)
                ->whereNotNull('embedding')
                ->count();
            $meta['chunk_schema_version'] = CodeSymbolAwareChunkBuilder::CHUNK_SCHEMA_VERSION;
            $meta['search_index_ready'] = app(CodeIndexRipgrepService::class)->isAvailable();
            $source->update([
                'status' => 'ready',
                'last_indexed_at' => now(),
                'indexing_progress' => 100,
                'indexing_phase' => null,
                'index_cancel_requested' => false,
                'metadata' => $meta,
            ]);

            $strategy = $useIncremental ? 'incremental' : ($budgetForcedFull ? 'full_chunk_budget_fallback' : 'full');
            $this->logCodeIndexOutcome($source->fresh(), [
                'strategy' => $strategy,
                'files_indexed' => count($newHashes),
                'chunks_total' => (int) ($meta['chunks_total'] ?? 0),
                'chunks_embedded' => (int) ($meta['chunks_embedded'] ?? 0),
                'chunks_inserted_this_run' => $inserted,
                'paths_refreshed_count' => $storedHashesBeforeStart !== null ? count($pathsToRefresh) : null,
                'paths_refreshed_sample' => $storedHashesBeforeStart !== null && $pathsToRefresh !== []
                    ? array_slice($pathsToRefresh, 0, 20)
                    : null,
                'scanned_files' => count($files),
                'max_total_chunks' => $maxChunks,
            ]);
        } catch (IndexingCancelledException) {
            $this->finishCancelled($source);
        } catch (\Throwable $e) {
            Log::error('code_index.failed', [
                'code_source_id' => $source->id,
                'exception' => $e::class,
                'error' => Utf8Text::forLog($e),
            ]);
            $source->update([
                'status' => 'failed',
                'error_message' => Utf8Text::sanitize(Utf8Text::forErrorColumn($e), 500),
                'indexing_progress' => 0,
                'indexing_phase' => null,
                'index_cancel_requested' => false,
            ]);
        }
    }

    /**
     * @param  array<int, string>  $excludePaths
     */
    protected function finalizeUnchangedIndex(VedaCodeSource $source, array $newHashes, ?int $scannedFilesCount = null, array $excludePaths = []): void
    {
        $this->reportProgress($source, 92, 'up_to_date');
        $total = (int) VedaCodeIndexChunk::query()->where('code_source_id', $source->id)->count();
        $embeddedAfter = (int) DB::table((new VedaCodeIndexChunk)->getTable())
            ->where('code_source_id', $source->id)
            ->whereNotNull('embedding')
            ->count();
        $source->update([
            'metadata' => $this->mergeCodeGraphMetadata($source, [
                'files_indexed' => count($newHashes),
                'chunks_total' => $total,
                'chunks_embedded' => $embeddedAfter,
                'code_index_file_hashes' => $newHashes,
                'exclude_paths' => $excludePaths,
                'workspace_ready' => true,
            ]),
            'status' => 'indexing',
        ]);
        $this->embedChunks($source->fresh());
        $this->throwIfCancelled($source->fresh());
        $fresh = $source->fresh();
        $meta = is_array($fresh?->metadata) ? $fresh->metadata : [];
        $meta['chunks_embedded'] = (int) DB::table((new VedaCodeIndexChunk)->getTable())
            ->where('code_source_id', $source->id)
            ->whereNotNull('embedding')
            ->count();
        $meta['chunk_schema_version'] = CodeSymbolAwareChunkBuilder::CHUNK_SCHEMA_VERSION;
        $meta['search_index_ready'] = app(CodeIndexRipgrepService::class)->isAvailable();
        $source->update([
            'status' => 'ready',
            'last_indexed_at' => now(),
            'indexing_progress' => 100,
            'indexing_phase' => null,
            'index_cancel_requested' => false,
            'metadata' => $meta,
        ]);
        $this->logCodeIndexOutcome($source->fresh(), [
            'strategy' => 'unchanged',
            'files_indexed' => count($newHashes),
            'chunks_total' => (int) ($meta['chunks_total'] ?? 0),
            'chunks_embedded' => (int) ($meta['chunks_embedded'] ?? 0),
            'chunks_inserted_this_run' => 0,
            'paths_refreshed_count' => 0,
            'paths_refreshed_sample' => null,
            'scanned_files' => $scannedFilesCount,
            'max_total_chunks' => (int) config('veda.code_index.max_total_chunks', 8000),
        ]);
    }

    protected function logCodeIndexOutcome(?VedaCodeSource $source, array $payload): void
    {
        if (! $source) {
            return;
        }
        Log::info('code_index.completed', array_merge([
            'code_source_id' => $source->id,
            'provider' => $source->provider,
        ], $payload));
    }

    protected function mergeCodeGraphMetadata(VedaCodeSource $source, array $metadata): array
    {
        $current = is_array($source->metadata) ? $source->metadata : [];

        return array_merge($current, $metadata);
    }

    protected function getStoredCodeIndexFileHashes(VedaCodeSource $source): ?array
    {
        $meta = $source->metadata;
        if (! is_array($meta) || ! array_key_exists('code_index_file_hashes', $meta)) {
            return null;
        }
        $hashes = $meta['code_index_file_hashes'];
        if (! is_array($hashes)) {
            return null;
        }

        return $hashes;
    }

    protected function distinctIndexedPaths(int $codeSourceId): array
    {
        return VedaCodeIndexChunk::query()
            ->where('code_source_id', $codeSourceId)
            ->distinct()
            ->orderBy('path')
            ->pluck('path')
            ->all();
    }

    protected function computeIndexableFileContentHashes(array $files): array
    {
        $maxBytes = (int) config('veda.code_index.max_file_bytes', 524288) + 64;
        $out = [];
        foreach ($files as $relPath => $absPath) {
            if ($this->shouldNeverIndexFile($relPath)) {
                continue;
            }
            $content = @file_get_contents($absPath);
            if ($content === false || $content === '') {
                continue;
            }
            $content = Utf8Text::sanitize($content, $maxBytes);
            if ($content === '') {
                continue;
            }
            if (! $this->isMostlyText($content)) {
                continue;
            }
            $sanPath = Utf8Text::sanitize($relPath, 2048);
            $out[$sanPath] = hash('sha256', $content);
        }
        ksort($out);

        return $out;
    }

    protected function computePathsToRefresh(array $storedHashes, array $newHashes, array $dbPaths): array
    {
        $refresh = [];
        $dbSet = array_fill_keys($dbPaths, true);
        $all = array_unique([...array_keys($storedHashes), ...array_keys($newHashes), ...$dbPaths]);
        foreach ($all as $path) {
            $o = $storedHashes[$path] ?? null;
            $n = $newHashes[$path] ?? null;
            if ($o !== $n) {
                $refresh[] = $path;

                continue;
            }
            if (isset($dbSet[$path]) && ! array_key_exists($path, $newHashes)) {
                $refresh[] = $path;
            }
        }
        sort($refresh);

        return array_values(array_unique($refresh));
    }

    protected function countChunksForSourcePaths(int $codeSourceId, array $paths): int
    {
        if ($paths === []) {
            return 0;
        }
        $total = 0;
        foreach (array_chunk($paths, 400) as $chunk) {
            $total += (int) DB::table((new VedaCodeIndexChunk)->getTable())
                ->where('code_source_id', $codeSourceId)
                ->whereIn('path', $chunk)
                ->count();
        }

        return $total;
    }

    protected function countChunksForPathsInFiles(array $files, array $pathsToRefresh, int $chunkMax): int
    {
        if ($pathsToRefresh === []) {
            return 0;
        }
        $allowed = array_fill_keys($pathsToRefresh, true);
        $total = 0;
        $maxBytes = (int) config('veda.code_index.max_file_bytes', 524288) + 64;
        foreach ($files as $relPath => $absPath) {
            if (! isset($allowed[$relPath])) {
                continue;
            }
            if ($this->shouldNeverIndexFile($relPath)) {
                continue;
            }
            $content = @file_get_contents($absPath);
            if ($content === false || $content === '') {
                continue;
            }
            $content = Utf8Text::sanitize($content, $maxBytes);
            if ($content === '') {
                continue;
            }
            if (! $this->isMostlyText($content)) {
                continue;
            }
            foreach (app(CodeSymbolAwareChunkBuilder::class)->lineChunks($content, $chunkMax) as $_) {
                $total++;
            }
        }

        return $total;
    }

    protected function collectReuseEmbeddingsMapForPaths(int $codeSourceId, array $paths): array
    {
        if ($paths === []) {
            return [];
        }
        $map = [];
        $driver = VedaCodeIndexChunk::query()->getConnection()->getDriverName();
        if (! in_array($driver, ['pgsql', 'sqlite'], true)) {
            return $map;
        }
        foreach (array_chunk($paths, 300) as $pathChunk) {
            $rows = DB::table((new VedaCodeIndexChunk)->getTable())
                ->where('code_source_id', $codeSourceId)
                ->whereIn('path', $pathChunk)
                ->whereNotNull('embedding')
                ->orderBy('id')
                ->get();
            foreach ($rows as $row) {
                $fp = $this->chunkContentFingerprint(
                    (string) $row->path,
                    (int) $row->start_line,
                    (int) $row->end_line,
                    (string) $row->content
                );
                $normalized = $this->normalizeStoredEmbeddingForInsert($row->embedding);
                if ($normalized !== null) {
                    $map[$fp] = $normalized;
                }
            }
        }

        return $map;
    }

    protected function appendChunksForFile(
        VedaCodeSource $source,
        string $relPath,
        string $absPath,
        int $chunkMax,
        array $reuseEmbeddings,
        int $newChunkRoom,
        int $insertedThisRun,
        array &$batch
    ): int {
        if ($this->shouldNeverIndexFile($relPath)) {
            return 0;
        }
        $content = @file_get_contents($absPath);
        if ($content === false || $content === '') {
            return 0;
        }
        $content = Utf8Text::sanitize($content, (int) config('veda.code_index.max_file_bytes', 524288) + 64);
        if ($content === '' || ! $this->isMostlyText($content)) {
            return 0;
        }
        $added = 0;
        $sanPath = Utf8Text::sanitize($relPath, 2048);
        $chunkBuilder = app(CodeSymbolAwareChunkBuilder::class);
        foreach ($chunkBuilder->lineChunks($content, $chunkMax) as $idx => $chunk) {
            $this->throwIfCancelled($source);
            if ($insertedThisRun + $added >= $newChunkRoom) {
                break;
            }
            $sanContent = Utf8Text::sanitize($chunk['text'], 4_194_304);
            $fp = $chunkBuilder->contentFingerprint($sanPath, $chunk['start'], $chunk['end'], $sanContent);
            $batch[] = [
                'code_source_id' => $source->id,
                'path' => $sanPath,
                'chunk_kind' => CodeSymbolAwareChunkBuilder::KIND_LINES,
                'chunk_index' => $idx,
                'start_line' => $chunk['start'],
                'end_line' => $chunk['end'],
                'content' => $sanContent,
                'content_hash' => $fp,
                'embedding' => $reuseEmbeddings[$fp] ?? null,
                'created_at' => now(),
                'updated_at' => now(),
            ];
            $added++;
            if (count($batch) >= 200) {
                DB::table((new VedaCodeIndexChunk)->getTable())->insert($batch);
                $batch = [];
            }
        }

        return $added;
    }

    public function ensureIndexingActive(VedaCodeSource $source): void
    {
        $this->throwIfCancelled($source);
    }

    public function isIndexingCancelled(int $codeSourceId): bool
    {
        $fresh = VedaCodeSource::query()->find($codeSourceId, ['status', 'index_cancel_requested']);
        if (! $fresh) {
            return true;
        }

        return $fresh->status === 'cancelled' || $fresh->index_cancel_requested;
    }

    protected function throwIfCancelled(VedaCodeSource $source): void
    {
        $fresh = $source->fresh();
        if (! $fresh) {
            throw new IndexingCancelledException;
        }
        if ($fresh->status === 'cancelled' || $fresh->index_cancel_requested) {
            throw new IndexingCancelledException;
        }
    }

    protected function reportProgress(VedaCodeSource $source, int $percent, string $phase): void
    {
        $source->update([
            'indexing_progress' => max(0, min(100, $percent)),
            'indexing_phase' => $phase,
        ]);
    }

    public function finishCancelled(VedaCodeSource $source): void
    {
        VedaCodeIndexChunk::where('code_source_id', $source->id)->delete();
        $source->update([
            'status' => 'cancelled',
            'error_message' => 'indexing_cancelled',
            'indexing_progress' => 0,
            'indexing_phase' => null,
            'index_cancel_requested' => false,
            'structure_summary' => null,
            'metadata' => null,
        ]);
    }

    public function resolveWorkspaceRoot(VedaCodeSource $source): ?string
    {
        if ($source->provider === 'local') {
            return $this->resolveLocalRoot($source);
        }

        if ($source->provider === 'github') {
            $this->throwIfCancelled($source);
            $this->reportProgress($source, 4, 'git_sync');
            $this->syncGitWorkspace($source);
            $this->throwIfCancelled($source);

            return storage_path('app/'.trim((string) $source->clone_relative_path, '/'));
        }

        return null;
    }

    protected function resolveLocalRoot(VedaCodeSource $source): ?string
    {
        $path = $source->local_absolute_path;
        if (! is_string($path) || trim($path) === '') {
            return null;
        }

        return $this->assertAllowedLocalDirectory($path);
    }

    /**
     * @throws \RuntimeException
     */
    public function assertAllowedLocalDirectory(string $path): string
    {
        if (! config('veda.code_index.allow_local_paths', false)) {
            throw new \RuntimeException('local_paths_disabled_set_allow_local_paths_true');
        }

        $real = realpath(trim($path));
        if ($real === false || ! is_dir($real)) {
            throw new \RuntimeException('local_path_not_found');
        }

        return $real;
    }

    /**
     * @return array<string, string>
     */
    public function collectIndexableFilesMap(string $root): array
    {
        return $this->collectFiles(rtrim($root, DIRECTORY_SEPARATOR));
    }

    protected function syncGitWorkspace(VedaCodeSource $source): void
    {
        $rel = trim((string) $source->clone_relative_path, '/');
        if ($rel === '') {
            throw new \RuntimeException('clone_path_missing');
        }

        $target = storage_path('app/'.$rel);

        [$token, $scheme] = $this->resolveGitCredentials($source);

        $url = $this->buildAuthenticatedGitUrl(
            (string) $source->git_remote_url,
            $token,
            $scheme
        );
        $branch = $source->git_branch ?: 'main';

        if (! is_dir($target.'/.git')) {
            $parent = dirname($target);
            if (! is_dir($parent)) {
                @mkdir($parent, 0755, true);
            }
            $tempTarget = $target.'_tmp_'.bin2hex(random_bytes(4));
            (new Process(['git', 'clone', '--depth', '1', '-b', $branch, $url, $tempTarget]))
                ->setTimeout(900)
                ->mustRun();
            if (is_dir($target)) {
                $this->removeDir($target);
            }
            if (! @rename($tempTarget, $target)) {
                $this->removeDir($tempTarget);
                throw new \RuntimeException('clone_rename_failed');
            }
        } else {
            (new Process(['git', '-C', $target, 'fetch', 'origin']))
                ->setTimeout(900)
                ->mustRun();
            (new Process(['git', '-C', $target, 'checkout', $branch]))
                ->setTimeout(120)
                ->run();
            (new Process(['git', '-C', $target, 'reset', '--hard', 'origin/'.$branch]))
                ->setTimeout(120)
                ->mustRun();
        }
    }

    /**
     * @return array{0: ?string, 1: 'pat_optional'|'github_installation'}
     */
    protected function resolveGitCredentials(VedaCodeSource $source): array
    {
        return [$source->git_clone_token, 'pat_optional'];
    }

    protected function buildAuthenticatedGitUrl(string $remoteUrl, ?string $token, string $credentialScheme = 'pat_optional'): string
    {
        $remoteUrl = trim($remoteUrl);
        if ($remoteUrl === '') {
            throw new \RuntimeException('git_remote_empty');
        }

        $parts = parse_url($remoteUrl);
        if (($parts['scheme'] ?? '') !== 'https') {
            throw new \RuntimeException('only_https_git_urls_supported');
        }

        $host = strtolower((string) ($parts['host'] ?? ''));
        $allowedHosts = config('veda.code_index.allowed_git_host_suffixes', []);
        $hostOk = false;
        foreach ($allowedHosts as $suffix) {
            $suffix = strtolower(trim((string) $suffix));
            if ($suffix !== '' && (str_ends_with($host, $suffix) || $host === $suffix)) {
                $hostOk = true;
                break;
            }
        }
        if (! $hostOk) {
            throw new \RuntimeException('git_host_not_allowed');
        }

        $tokenPlain = is_string($token) ? trim($token) : '';
        $path = ($parts['path'] ?? '').(isset($parts['query']) ? '?'.$parts['query'] : '');
        if ($credentialScheme === 'github_installation') {
            if ($tokenPlain === '') {
                throw new \RuntimeException('github_integration_failed');
            }

            return 'https://x-access-token:'.rawurlencode($tokenPlain).'@'.$host.$path;
        }

        if ($tokenPlain === '') {
            return $remoteUrl;
        }

        return 'https://oauth2:'.rawurlencode($tokenPlain).'@'.$host.$path;
    }

    /**
     * @return array<string, mixed>
     */
    public function estimateIndexFootprint(string $root, ?array $excludePaths = null): array
    {
        $real = realpath($root);
        if ($real === false || ! is_dir($real)) {
            throw new \RuntimeException('local_path_not_found');
        }
        $rootNorm = rtrim($real, DIRECTORY_SEPARATOR);
        $files = $this->collectFiles($rootNorm, null, $excludePaths);
        $chunkMax = config('veda.code_index.chunk_max_chars', 1600);
        $maxChunks = config('veda.code_index.max_total_chunks', 8000);
        $embeddingService = app(CodeIndexEmbeddingService::class);
        $totalChunks = 0;
        $totalEmbeddingChars = 0;
        $filesIndexable = 0;
        $capped = false;

        foreach ($files as $rel => $abs) {
            if ($this->shouldNeverIndexFile($rel)) {
                continue;
            }
            $content = @file_get_contents($abs);
            if ($content === false || $content === '') {
                continue;
            }
            if (! $this->isMostlyText($content)) {
                continue;
            }
            $filesIndexable++;
            foreach (app(CodeSymbolAwareChunkBuilder::class)->lineChunks($content, $chunkMax) as $chunk) {
                if ($totalChunks >= $maxChunks) {
                    $capped = true;
                    break 2;
                }
                $text = $this->textForCodeChunkEmbedding($rel, $chunk['start'], $chunk['end'], $chunk['text']);
                $text = $embeddingService->clampEmbeddingInput($text);
                $totalEmbeddingChars += mb_strlen($text);
                $totalChunks++;
            }
        }

        $charsPerToken = (float) config('veda.code_index.embedding_chars_per_token_estimate', 3.5);
        $tokens = $charsPerToken > 0 ? (int) ceil($totalEmbeddingChars / $charsPerToken) : 0;
        $rubPerM = (float) config('veda.code_index.embedding_input_rub_per_million', 45);
        $costRub = ($tokens / 1_000_000) * $rubPerM;

        return [
            'files_scanned' => count($files),
            'files_indexable' => $filesIndexable,
            'chunks_total' => $totalChunks,
            'embedding_chars_total' => $totalEmbeddingChars,
            'estimated_input_tokens' => $tokens,
            'estimated_cost_rub' => round($costRub, 2),
            'rate_rub_per_million_tokens' => $rubPerM,
            'capped_by_max_chunks' => $capped,
        ];
    }

    protected function rootGitignoreRegex(string $root): ?string
    {
        $path = $root.DIRECTORY_SEPARATOR.'.gitignore';
        if (! is_file($path)) {
            return null;
        }
        $raw = @file_get_contents($path);
        if ($raw === false || trim($raw) === '') {
            return null;
        }
        try {
            return Gitignore::toRegex($raw);
        } catch (\Throwable) {
            return null;
        }
    }

    /**
     * @return array<string, string>
     */
    /**
     * @return array<int, string>
     */
    public function userExcludePaths(VedaCodeSource $source): array
    {
        $meta = is_array($source->metadata) ? $source->metadata : [];
        $raw = $meta['exclude_paths'] ?? [];

        return $this->normalizeExcludePaths(is_array($raw) ? $raw : []);
    }

    /**
     * @param  array<int, mixed>  $paths
     * @return array<int, string>
     */
    public function normalizeExcludePaths(array $paths): array
    {
        $out = [];
        foreach ($paths as $path) {
            $path = trim(str_replace('\\', '/', (string) $path), '/');
            if ($path === '' || str_contains($path, '..')) {
                continue;
            }
            $out[$path] = true;
        }

        return array_keys($out);
    }

    public function isPathExcludedByUserConfig(string $rel, array $excludePaths): bool
    {
        if ($excludePaths === []) {
            return false;
        }
        $rel = str_replace('\\', '/', $rel);
        foreach ($excludePaths as $excluded) {
            if ($rel === $excluded || str_starts_with($rel, $excluded.'/')) {
                return true;
            }
        }

        return false;
    }

    public function prepareWorkspace(VedaCodeSource $source): void
    {
        $source->refresh();
        if ($source->status !== 'configuring') {
            return;
        }

        try {
            if ($source->provider === 'github') {
                $source->update(['indexing_phase' => 'git_sync']);
                $this->syncGitWorkspace($source);
            } elseif ($source->provider === 'local') {
                $this->resolveLocalRoot($source);
            } else {
                throw new \RuntimeException('workspace_not_resolved');
            }

            $meta = is_array($source->metadata) ? $source->metadata : [];
            $meta['workspace_ready'] = true;
            if (! isset($meta['exclude_paths']) || ! is_array($meta['exclude_paths'])) {
                $meta['exclude_paths'] = [];
            }
            $source->update([
                'indexing_phase' => null,
                'metadata' => $meta,
            ]);
        } catch (\Throwable $e) {
            $msg = $e instanceof \RuntimeException ? $e->getMessage() : 'queue_job_failed';
            Log::warning('code_index.prepare_workspace_failed', [
                'code_source_id' => $source->id,
                'detail' => $msg,
            ]);
            $source->update([
                'status' => 'failed',
                'error_message' => Utf8Text::sanitize($msg, 500),
                'indexing_phase' => null,
                'indexing_progress' => 0,
            ]);
        }
    }

    /**
     * @return array<int, array{name: string, path: string, kind: string}>
     */
    public function listScopeChildren(VedaCodeSource $source, string $parentPath = ''): array
    {
        $root = $this->resolveScopeWorkspaceRoot($source);
        $parentPath = $this->normalizeScopeParentPath($parentPath);

        if (is_dir($root.DIRECTORY_SEPARATOR.'.git')) {
            return $this->listScopeChildrenUsingGit($root, $parentPath);
        }

        return $this->listScopeChildrenFilesystem($root, $parentPath);
    }

    public function normalizeScopeParentPath(string $parentPath): string
    {
        $parentPath = trim(str_replace('\\', '/', $parentPath), '/');
        if (str_contains($parentPath, '..')) {
            throw new \RuntimeException('scope_path_invalid');
        }

        return $parentPath;
    }

    /**
     * @return array<int, array{name: string, path: string, kind: string}>
     */
    protected function listScopeChildrenFilesystem(string $root, string $parentPath): array
    {
        $rootReal = realpath($root);
        if ($rootReal === false) {
            throw new \RuntimeException('workspace_not_resolved');
        }

        $dir = $parentPath === ''
            ? $rootReal
            : $rootReal.DIRECTORY_SEPARATOR.str_replace('/', DIRECTORY_SEPARATOR, $parentPath);
        $dirReal = realpath($dir);
        if ($dirReal === false || ! is_dir($dirReal) || ! $this->isPathInsideRoot($rootReal, $dirReal)) {
            throw new \RuntimeException('scope_path_not_found');
        }

        $maxBytes = config('veda.code_index.max_file_bytes', 524288);
        $entries = [];

        foreach (scandir($dirReal) ?: [] as $name) {
            if ($name === '.' || $name === '..') {
                continue;
            }
            $rel = $parentPath === '' ? $name : $parentPath.'/'.$name;
            $full = $dirReal.DIRECTORY_SEPARATOR.$name;

            if (is_dir($full)) {
                if ($this->shouldSkipPath($rel.'/placeholder')) {
                    continue;
                }
                $entries[] = ['name' => $name, 'path' => $rel, 'kind' => 'dir'];
            } elseif (is_file($full)) {
                if ($this->shouldNeverIndexFile($rel) || ! $this->isAllowedExtension($rel)) {
                    continue;
                }
                if (@filesize($full) > $maxBytes) {
                    continue;
                }
                $entries[] = ['name' => $name, 'path' => $rel, 'kind' => 'file'];
            }
        }

        return $this->sortScopeEntries($entries);
    }

    /**
     * @return array<int, array{name: string, path: string, kind: string}>
     */
    protected function listScopeChildrenUsingGit(string $root, string $parentPath): array
    {
        $root = rtrim($root, DIRECTORY_SEPARATOR);
        $treeRef = $parentPath === '' ? 'HEAD' : 'HEAD:'.$parentPath;
        $process = new Process(['git', '-C', $root, 'ls-tree', $treeRef]);
        $process->setTimeout(120);
        $process->run();
        if (! $process->isSuccessful()) {
            throw new \RuntimeException('scope_path_not_found');
        }

        $maxBytes = config('veda.code_index.max_file_bytes', 524288);
        $entries = [];
        foreach (preg_split('/\r\n|\r|\n/', trim($process->getOutput())) as $line) {
            if ($line === '') {
                continue;
            }
            if (! preg_match('/^\d+\s+(blob|tree|commit)\s+[a-f0-9]+\s+(.+)$/', $line, $matches)) {
                continue;
            }
            $objectType = $matches[1];
            $name = $matches[2];
            if (str_contains($name, '/')) {
                continue;
            }
            $rel = $parentPath === '' ? $name : $parentPath.'/'.$name;

            if ($objectType === 'tree' || $objectType === 'commit') {
                if ($this->shouldSkipPath($rel.'/placeholder')) {
                    continue;
                }
                $entries[] = ['name' => $name, 'path' => $rel, 'kind' => 'dir'];
            } elseif ($objectType === 'blob') {
                if ($this->shouldNeverIndexFile($rel) || ! $this->isAllowedExtension($rel)) {
                    continue;
                }
                $abs = $root.DIRECTORY_SEPARATOR.str_replace('/', DIRECTORY_SEPARATOR, $rel);
                if (is_file($abs) && @filesize($abs) > $maxBytes) {
                    continue;
                }
                $entries[] = ['name' => $name, 'path' => $rel, 'kind' => 'file'];
            }
        }

        return $this->sortScopeEntries($entries);
    }

    /**
     * @param  array<int, array{name: string, path: string, kind: string}>  $entries
     * @return array<int, array{name: string, path: string, kind: string}>
     */
    protected function sortScopeEntries(array $entries): array
    {
        usort($entries, function (array $a, array $b): int {
            if ($a['kind'] !== $b['kind']) {
                return $a['kind'] === 'dir' ? -1 : 1;
            }

            return strcasecmp($a['name'], $b['name']);
        });

        return $entries;
    }

    protected function isPathInsideRoot(string $rootReal, string $pathReal): bool
    {
        $rootNorm = rtrim(str_replace('\\', '/', $rootReal), '/');
        $pathNorm = rtrim(str_replace('\\', '/', $pathReal), '/');

        return $pathNorm === $rootNorm || str_starts_with($pathNorm.'/', $rootNorm.'/');
    }

    public function estimateIndexFootprintForSource(VedaCodeSource $source, ?array $excludePaths = null): array
    {
        $root = $this->resolveScopeWorkspaceRoot($source);
        $paths = $excludePaths !== null
            ? $this->normalizeExcludePaths($excludePaths)
            : $this->userExcludePaths($source);

        return $this->estimateIndexFootprint($root, $paths);
    }

    /**
     * @throws \RuntimeException
     */
    public function resolveScopeWorkspaceRoot(VedaCodeSource $source): string
    {
        if ($source->provider === 'local') {
            $root = $this->resolveLocalRoot($source);
            if ($root === null) {
                throw new \RuntimeException('workspace_not_resolved');
            }

            return $root;
        }

        if ($source->provider === 'github') {
            $meta = is_array($source->metadata) ? $source->metadata : [];
            if (empty($meta['workspace_ready'])) {
                throw new \RuntimeException('workspace_not_ready');
            }
            $rel = trim((string) $source->clone_relative_path, '/');
            if ($rel === '') {
                throw new \RuntimeException('clone_path_missing');
            }
            $target = storage_path('app/'.$rel);
            if (! is_dir($target)) {
                throw new \RuntimeException('workspace_not_ready');
            }

            return rtrim($target, DIRECTORY_SEPARATOR);
        }

        throw new \RuntimeException('workspace_not_resolved');
    }

    protected function collectFiles(string $root, ?VedaCodeSource $progressSource = null, ?array $excludePaths = null): array
    {
        $root = rtrim($root, DIRECTORY_SEPARATOR);
        if ($excludePaths === null && $progressSource !== null) {
            $excludePaths = $this->userExcludePaths($progressSource);
        }
        $excludePaths ??= [];

        if (is_dir($root.DIRECTORY_SEPARATOR.'.git')) {
            return $this->collectFilesUsingGit($root, $progressSource, $excludePaths);
        }

        return $this->collectFilesFilesystem($root, $progressSource, $excludePaths);
    }

    /**
     * @return array<string, string>
     */
    /**
     * @param  array<int, string>  $excludePaths
     * @return array<string, string>
     */
    protected function collectFilesUsingGit(string $root, ?VedaCodeSource $progressSource = null, array $excludePaths = []): array
    {
        $root = rtrim($root, DIRECTORY_SEPARATOR);
        $maxFiles = config('veda.code_index.max_files_indexed', 5000);
        $maxBytes = config('veda.code_index.max_file_bytes', 524288);
        $process = new Process(['git', '-C', $root, 'ls-files', '-z', '--cached', '--others', '--exclude-standard']);
        $process->setTimeout(180);
        $process->run();
        if (! $process->isSuccessful()) {
            return $this->collectFilesFilesystem($root, $progressSource, $excludePaths);
        }

        $raw = $process->getOutput();
        $rels = $raw === '' ? [] : explode("\0", rtrim($raw, "\0"));
        $out = [];
        $seen = 0;
        foreach ($rels as $rel) {
            $rel = str_replace('\\', '/', $rel);
            if ($rel === '' || str_ends_with($rel, '/')) {
                continue;
            }
            if ($progressSource !== null && $seen > 0 && $seen % 80 === 0) {
                $this->throwIfCancelled($progressSource);
                $n = count($out);
                $pct = 10 + (int) min(22, ($n / max(1, $maxFiles)) * 22);
                $this->reportProgress($progressSource, $pct, 'scan_files');
            }
            if (count($out) >= $maxFiles) {
                break;
            }
            $abs = $root.DIRECTORY_SEPARATOR.str_replace('/', DIRECTORY_SEPARATOR, $rel);
            if (! is_file($abs)) {
                continue;
            }
            if ($this->shouldNeverIndexFile($rel)) {
                continue;
            }
            if ($this->shouldSkipPath($rel)) {
                continue;
            }
            if ($this->isPathExcludedByUserConfig($rel, $excludePaths)) {
                continue;
            }
            if (@filesize($abs) > $maxBytes) {
                continue;
            }
            if (! $this->isAllowedExtension($rel)) {
                continue;
            }
            $out[$rel] = $abs;
            $seen++;
        }

        ksort($out);

        if ($progressSource !== null) {
            $this->reportProgress($progressSource, 33, 'scan_files');
        }

        return $out;
    }

    /**
     * @param  array<int, string>  $excludePaths
     * @return array<string, string>
     */
    protected function collectFilesFilesystem(string $root, ?VedaCodeSource $progressSource = null, array $excludePaths = []): array
    {
        $root = rtrim($root, DIRECTORY_SEPARATOR);
        $maxFiles = config('veda.code_index.max_files_indexed', 5000);
        $maxBytes = config('veda.code_index.max_file_bytes', 524288);
        $gitignoreRegex = $this->rootGitignoreRegex($root);
        $out = [];
        $iterator = new \RecursiveIteratorIterator(
            new \RecursiveDirectoryIterator($root, \FilesystemIterator::SKIP_DOTS)
        );

        foreach ($iterator as $file) {
            if ($progressSource !== null && count($out) > 0 && count($out) % 80 === 0) {
                $this->throwIfCancelled($progressSource);
                $n = count($out);
                $pct = 10 + (int) min(22, ($n / max(1, $maxFiles)) * 22);
                $this->reportProgress($progressSource, $pct, 'scan_files');
            }
            if (count($out) >= $maxFiles) {
                break;
            }
            if (! $file->isFile()) {
                continue;
            }
            $abs = $file->getPathname();
            $rel = ltrim(str_replace($root, '', $abs), DIRECTORY_SEPARATOR);
            $rel = str_replace(DIRECTORY_SEPARATOR, '/', $rel);
            if ($this->shouldNeverIndexFile($rel)) {
                continue;
            }
            if ($this->shouldSkipPath($rel)) {
                continue;
            }
            if ($this->isPathExcludedByUserConfig($rel, $excludePaths)) {
                continue;
            }
            if ($gitignoreRegex !== null && @preg_match($gitignoreRegex, $rel) === 1) {
                continue;
            }
            if ($file->getSize() > $maxBytes) {
                continue;
            }
            if (! $this->isAllowedExtension($rel)) {
                continue;
            }
            $out[$rel] = $abs;
        }

        ksort($out);

        if ($progressSource !== null) {
            $this->reportProgress($progressSource, 33, 'scan_files');
        }

        return $out;
    }

    protected function shouldNeverIndexFile(string $rel): bool
    {
        $rel = str_replace('\\', '/', $rel);
        $base = basename($rel);
        if ($base === '.env') {
            return true;
        }
        if ($base === '.env.example') {
            return false;
        }
        if (str_starts_with($base, '.env.')) {
            return true;
        }
        $lower = strtolower($rel);
        if (str_ends_with($lower, '.env') && ! str_ends_with($lower, '.env.example')) {
            return true;
        }

        return false;
    }

    protected function shouldSkipPath(string $rel): bool
    {
        $lower = strtolower($rel);
        foreach (self::SKIP_DIR_NAMES as $dir) {
            $d = strtolower($dir);
            if (str_contains($lower, '/'.$d.'/') || str_starts_with($lower, $d.'/')) {
                return true;
            }
        }

        return false;
    }

    protected function isAllowedExtension(string $rel): bool
    {
        if ($this->shouldNeverIndexFile($rel)) {
            return false;
        }
        foreach (self::EXTENSIONS as $ext) {
            if (str_ends_with(strtolower($rel), '.'.$ext)) {
                return true;
            }
        }

        return false;
    }

    protected function isMostlyText(string $binary): bool
    {
        if ($binary === '') {
            return false;
        }
        if (str_contains($binary, "\0")) {
            return false;
        }
        $sample = substr($binary, 0, 8000);
        $len = strlen($sample);
        if ($len === 0) {
            return false;
        }
        $printable = 0;
        for ($i = 0; $i < $len; $i++) {
            $c = $sample[$i];
            $o = ord($c);
            if ($o === 9 || $o === 10 || $o === 13 || ($o >= 32 && $o <= 126) || $o >= 128) {
                $printable++;
            }
        }

        return ($printable / $len) >= 0.92;
    }

    /**
     * @return list<array{start:int,end:int,text:string}>
     */
    protected function chunkContent(string $content, int $maxChars): array
    {
        $lines = preg_split('/\R/', $content);
        if (! is_array($lines)) {
            return [['start' => 1, 'end' => 1, 'text' => mb_substr($content, 0, $maxChars)]];
        }

        $chunks = [];
        $buf = '';
        $startLine = 1;
        $lineNo = 0;
        foreach ($lines as $line) {
            $lineNo++;
            $candidate = $buf === '' ? $line : $buf."\n".$line;
            if (strlen($candidate) > $maxChars && $buf !== '') {
                $chunks[] = [
                    'start' => $startLine,
                    'end' => $lineNo - 1,
                    'text' => $buf,
                ];
                $buf = $line;
                $startLine = $lineNo;
            } else {
                $buf = $candidate;
            }
        }
        if ($buf !== '') {
            $chunks[] = [
                'start' => $startLine,
                'end' => $lineNo,
                'text' => $buf,
            ];
        }

        return $chunks === [] ? [['start' => 1, 'end' => 1, 'text' => '']] : $chunks;
    }

    protected function truncatePathForEmbedding(string $path): string
    {
        $max = 448;
        if (mb_strlen($path) <= $max) {
            return $path;
        }

        return mb_substr($path, 0, $max - 1).'…';
    }

    protected function textForCodeChunkEmbedding(string $path, int $startLine, int $endLine, string $content): string
    {
        $pathDisplay = $this->truncatePathForEmbedding($path);

        return 'File: '.$pathDisplay.' lines '.$startLine.'-'.$endLine."\n".$content;
    }

    protected function chunkContentFingerprint(string $path, int $startLine, int $endLine, string $content): string
    {
        return app(CodeSymbolAwareChunkBuilder::class)->contentFingerprint($path, $startLine, $endLine, $content);
    }

    /**
     * @return array<string, string>
     */
    protected function collectReuseEmbeddingsMap(int $codeSourceId): array
    {
        $map = [];
        $driver = VedaCodeIndexChunk::query()->getConnection()->getDriverName();
        if (! in_array($driver, ['pgsql', 'sqlite'], true)) {
            return $map;
        }

        DB::table((new VedaCodeIndexChunk)->getTable())
            ->where('code_source_id', $codeSourceId)
            ->whereNotNull('embedding')
            ->orderBy('id')
            ->chunkById(500, function ($rows) use (&$map): void {
                foreach ($rows as $row) {
                    $fp = $this->chunkContentFingerprint(
                        (string) $row->path,
                        (int) $row->start_line,
                        (int) $row->end_line,
                        (string) $row->content
                    );
                    $normalized = $this->normalizeStoredEmbeddingForInsert($row->embedding);
                    if ($normalized !== null) {
                        $map[$fp] = $normalized;
                    }
                }
            });

        return $map;
    }

    protected function normalizeStoredEmbeddingForInsert(mixed $stored): ?string
    {
        if ($stored === null) {
            return null;
        }
        if (is_string($stored)) {
            $s = trim($stored);

            return $s === '' ? null : $s;
        }
        if (is_array($stored)) {
            return json_encode($stored);
        }

        return null;
    }

    protected function reportEmbeddingProgress(VedaCodeSource $source, int $embeddedCount, int $totalChunks): void
    {
        $source->refresh();
        $meta = is_array($source->metadata) ? $source->metadata : [];
        $meta['chunks_total'] = $totalChunks;
        $meta['chunks_embedded'] = $embeddedCount;
        if ($totalChunks <= 0) {
            $source->update([
                'indexing_progress' => 100,
                'indexing_phase' => 'embedding',
                'metadata' => $meta,
            ]);

            return;
        }
        $pct = min(99, max(74, 74 + (int) floor(25 * $embeddedCount / $totalChunks)));
        $source->update([
            'indexing_progress' => $pct,
            'indexing_phase' => 'embedding',
            'metadata' => $meta,
        ]);
    }

    protected function embedChunks(?VedaCodeSource $source): void
    {
        if (! $source) {
            return;
        }

        $embeddingService = app(CodeIndexEmbeddingService::class);
        $total = (int) VedaCodeIndexChunk::query()->where('code_source_id', $source->id)->count();
        if ($total === 0) {
            $this->reportEmbeddingProgress($source, 0, 0);

            return;
        }

        $embedded = (int) VedaCodeIndexChunk::query()
            ->where('code_source_id', $source->id)
            ->whereNotNull('embedding')
            ->count();
        $this->reportEmbeddingProgress($source, $embedded, $total);

        if (! $embeddingService->isConfigured()) {
            Log::warning('code_index.embeddings_skipped', [
                'code_source_id' => $source->id,
                'reason' => 'embedding_service_not_configured',
            ]);

            return;
        }

        if (! $embeddingService->isAvailable()) {
            Log::warning('code_index.embeddings_skipped', [
                'code_source_id' => $source->id,
                'reason' => 'embedding_circuit_open',
            ]);

            return;
        }

        VedaCodeIndexChunk::query()
            ->where('code_source_id', $source->id)
            ->whereNull('embedding')
            ->orderBy('id')
            ->chunkById(40, function ($chunks) use ($embeddingService, $source, $total): bool {
                foreach ($chunks as $chunk) {
                    $this->throwIfCancelled($source);
                    $builder = app(CodeSymbolAwareChunkBuilder::class);
                    $text = $builder->embeddingText(
                        (string) $chunk->path,
                        (int) $chunk->start_line,
                        (int) $chunk->end_line,
                        (string) $chunk->content,
                        $chunk->qualified_name ?? null,
                        is_string($chunk->language ?? null) ? $chunk->language : null,
                    );
                    $text = app(CodeIndexEmbeddingService::class)->clampEmbeddingInput($text);
                    $vec = $embeddingService->createDocumentEmbedding($text, 3);
                    if ($vec) {
                        DB::table((new VedaCodeIndexChunk)->getTable())
                            ->where('id', $chunk->id)
                            ->update(['embedding' => json_encode($vec)]);
                    } elseif (! $embeddingService->isAvailable()) {
                        Log::warning('code_index.embeddings_aborted', [
                            'code_source_id' => $source->id,
                            'reason' => 'embedding_circuit_open',
                        ]);

                        return false;
                    }
                }
                $embeddedNow = (int) VedaCodeIndexChunk::query()
                    ->where('code_source_id', $source->id)
                    ->whereNotNull('embedding')
                    ->count();
                $this->reportEmbeddingProgress($source, $embeddedNow, $total);

                return true;
            });
    }

    protected function removeDir(string $dir): void
    {
        if (! is_dir($dir)) {
            return;
        }
        $it = new \RecursiveDirectoryIterator($dir, \FilesystemIterator::SKIP_DOTS);
        $files = new \RecursiveIteratorIterator($it, \RecursiveIteratorIterator::CHILD_FIRST);
        foreach ($files as $file) {
            if ($file->isDir()) {
                @rmdir($file->getPathname());
            } else {
                @unlink($file->getPathname());
            }
        }
        @rmdir($dir);
    }
}
