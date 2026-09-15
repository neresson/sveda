<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Storage;
use Illuminate\Validation\Rule;
use Illuminate\Validation\ValidationException;
use RuntimeException;
use Veda\Laravel\CodeIndex\CodeSourceIndexer;
use Veda\Laravel\Jobs\IndexCodeSourceJob;
use Veda\Laravel\Jobs\PrepareCodeSourceWorkspaceJob;
use Veda\Laravel\Models\VedaCodeSource;

class VedaCodeIndexController
{
    public function index(CodeSourceIndexer $indexer): JsonResponse
    {
        return response()->json([
            'sources' => $this->sourceList(),
            'localIndexingEnabled' => $this->localIndexingEnabled(),
        ]);
    }

    public function progress(): JsonResponse
    {
        $sources = VedaCodeSource::query()
            ->orderByDesc('updated_at')
            ->get([
                'id', 'status', 'indexing_progress', 'indexing_phase', 'metadata',
                'error_message', 'last_indexed_at', 'updated_at',
            ]);

        return response()->json(['sources' => $sources]);
    }

    public function store(Request $request, CodeSourceIndexer $indexer): JsonResponse
    {
        $validated = $request->validate([
            'name' => 'required|string|max:255',
            'provider' => 'required|in:local,github',
            'description' => 'nullable|string|max:8000',
            'local_absolute_path' => [
                Rule::requiredIf($request->input('provider') === 'local'),
                'nullable',
                'string',
                'max:2048',
            ],
            'git_remote_url' => [
                Rule::requiredIf($request->input('provider') === 'github'),
                'nullable',
                'string',
                'max:2048',
            ],
            'git_branch' => 'nullable|string|max:190',
            'git_clone_token' => 'nullable|string|max:500',
        ]);

        $validated['git_branch'] = isset($validated['git_branch'])
            ? trim((string) $validated['git_branch'])
            : '';
        $validated['status'] = 'configuring';
        $validated['metadata'] = [
            'exclude_paths' => [],
            'workspace_ready' => false,
        ];

        if ($validated['provider'] === 'local') {
            try {
                $validated['local_absolute_path'] = $indexer->assertAllowedLocalDirectory($validated['local_absolute_path']);
            } catch (RuntimeException $e) {
                throw ValidationException::withMessages([
                    'local_absolute_path' => [$e->getMessage()],
                ]);
            }
            $validated['git_remote_url'] = null;
            $validated['git_clone_token'] = null;
            $validated['clone_relative_path'] = null;
        } else {
            $validated['local_absolute_path'] = null;
        }

        if (($validated['git_branch'] ?? '') === '') {
            $validated['git_branch'] = 'main';
        }

        $source = VedaCodeSource::query()->create($validated);

        if ($source->provider === 'github') {
            $source->clone_relative_path = 'code-index/'.$source->id;
            $source->save();
        }

        PrepareCodeSourceWorkspaceJob::dispatch($source->id);

        return response()->json([
            'source' => $this->serializeSource($source->fresh() ?? $source),
            'sources' => $this->sourceList(),
        ], 201);
    }

    public function update(Request $request, VedaCodeSource $codeSource): JsonResponse
    {
        $validated = $request->validate([
            'name' => 'required|string|max:255',
            'description' => 'nullable|string|max:8000',
            'exclude_paths' => 'nullable|array|max:500',
            'exclude_paths.*' => 'string|max:512',
            'start_indexing' => 'sometimes|boolean',
        ]);

        $indexer = app(CodeSourceIndexer::class);
        $meta = is_array($codeSource->metadata) ? $codeSource->metadata : [];
        $meta['exclude_paths'] = $indexer->normalizeExcludePaths($validated['exclude_paths'] ?? $meta['exclude_paths'] ?? []);

        $updates = [
            'name' => $validated['name'],
            'description' => $validated['description'] ?? null,
            'metadata' => $meta,
        ];

        $startIndexing = (bool) ($validated['start_indexing'] ?? false);
        if ($startIndexing && $codeSource->status === 'configuring') {
            $workspaceReady = $codeSource->provider === 'local'
                || ! empty($meta['workspace_ready']);
            if (! $workspaceReady) {
                throw ValidationException::withMessages([
                    'exclude_paths' => ['workspace_not_ready'],
                ]);
            }
            $updates['status'] = 'pending';
            $updates['error_message'] = null;
            $updates['indexing_progress'] = 0;
            $updates['indexing_phase'] = null;
            $updates['index_cancel_requested'] = false;
        }

        $codeSource->update($updates);

        if ($startIndexing && $codeSource->fresh()?->status === 'pending') {
            IndexCodeSourceJob::dispatch($codeSource->id);
        }

        return response()->json([
            'source' => $this->serializeSource($codeSource->fresh() ?? $codeSource),
            'sources' => $this->sourceList(),
        ]);
    }

    public function destroy(VedaCodeSource $codeSource): JsonResponse
    {
        $rel = trim((string) $codeSource->clone_relative_path, '/');
        if ($rel !== '' && $codeSource->provider === 'github') {
            Storage::disk('local')->deleteDirectory($rel);
        }

        $codeSource->delete();

        return response()->json(['sources' => $this->sourceList()]);
    }

    public function cancel(Request $request, VedaCodeSource $codeSource, CodeSourceIndexer $indexer): JsonResponse
    {
        if (! in_array($codeSource->status, ['pending', 'indexing', 'configuring'], true)) {
            return response()->json(['ok' => false], 422);
        }

        $codeSource->update([
            'index_cancel_requested' => true,
            'status' => 'cancelled',
            'indexing_phase' => null,
            'error_message' => 'indexing_cancelled',
        ]);

        IndexCodeSourceJob::releaseOverlapLock($codeSource->id);
        $codeSource->refresh();
        $indexer->finishCancelled($codeSource);

        return response()->json([
            'ok' => true,
            'source' => $this->serializeSource($codeSource->fresh() ?? $codeSource),
            'sources' => $this->sourceList(),
        ]);
    }

    public function reindex(VedaCodeSource $codeSource): JsonResponse
    {
        $codeSource->refresh();

        if (in_array($codeSource->status, ['pending', 'indexing'], true)) {
            return response()->json([
                'ok' => false,
                'error' => 'reindex_skipped_already_running',
                'source' => $this->serializeSource($codeSource),
            ], 422);
        }

        if (in_array($codeSource->status, ['failed', 'cancelled'], true)) {
            $codeSource->update([
                'status' => 'pending',
                'error_message' => null,
                'indexing_progress' => 0,
                'indexing_phase' => null,
                'index_cancel_requested' => false,
            ]);
        }

        IndexCodeSourceJob::dispatch($codeSource->id);

        return response()->json([
            'ok' => true,
            'source' => $this->serializeSource($codeSource->fresh() ?? $codeSource),
            'sources' => $this->sourceList(),
        ]);
    }

    public function browse(Request $request, CodeSourceIndexer $indexer): JsonResponse
    {
        $path = $request->query('path', '');
        $path = is_string($path) ? trim($path) : '';

        try {
            if ($path === '') {
                if (! $this->localIndexingEnabled()) {
                    throw new RuntimeException('local_paths_disabled_set_allow_local_paths_true');
                }

                return response()->json([
                    'current_path' => null,
                    'parent_path' => null,
                    'entries' => [],
                    'needs_anchor' => true,
                ]);
            }

            $real = $indexer->assertAllowedLocalDirectory($path);

            return response()->json($this->browseDirectory($indexer, $real));
        } catch (RuntimeException $e) {
            return response()->json(['message' => $e->getMessage()], 422);
        }
    }

    public function preview(Request $request, CodeSourceIndexer $indexer): JsonResponse
    {
        $data = $request->validate([
            'path' => 'required|string|max:2048',
        ]);

        try {
            $real = $indexer->assertAllowedLocalDirectory($data['path']);
            $map = $indexer->collectIndexableFilesMap($real);
            $paths = array_keys($map);
            $total = count($paths);
            $maxFiles = (int) config('veda.code_index.max_files_indexed', 5000);
            $sampleLimit = 200;
            $sample = array_slice($paths, 0, $sampleLimit);

            return response()->json([
                'path' => $real,
                'total_files' => $total,
                'sample_paths' => $sample,
                'truncated_sample' => $total > $sampleLimit,
                'hit_index_cap' => $total >= $maxFiles,
                'max_files' => $maxFiles,
            ]);
        } catch (RuntimeException $e) {
            return response()->json(['message' => $e->getMessage()], 422);
        }
    }

    public function estimateFootprint(Request $request, CodeSourceIndexer $indexer): JsonResponse
    {
        $data = $request->validate([
            'path' => 'required|string|max:2048',
        ]);

        try {
            $real = $indexer->assertAllowedLocalDirectory($data['path']);

            return response()->json($indexer->estimateIndexFootprint($real));
        } catch (RuntimeException $e) {
            return response()->json(['message' => $e->getMessage()], 422);
        }
    }

    public function scope(Request $request, VedaCodeSource $codeSource, CodeSourceIndexer $indexer): JsonResponse
    {
        $meta = is_array($codeSource->metadata) ? $codeSource->metadata : [];
        $workspaceReady = $codeSource->provider === 'local'
            || ! empty($meta['workspace_ready']);
        $excludePaths = $indexer->userExcludePaths($codeSource);

        if (! $workspaceReady) {
            return response()->json([
                'ready' => false,
                'parent' => '',
                'entries' => [],
                'exclude_paths' => $excludePaths,
                'indexing_phase' => $codeSource->indexing_phase,
            ]);
        }

        $parent = $request->query('parent', '');
        $parent = is_string($parent) ? $parent : '';

        try {
            $parent = $indexer->normalizeScopeParentPath($parent);
            $entries = $indexer->listScopeChildren($codeSource, $parent);

            return response()->json([
                'ready' => true,
                'parent' => $parent,
                'entries' => $entries,
                'exclude_paths' => $excludePaths,
            ]);
        } catch (RuntimeException $e) {
            return response()->json([
                'ready' => false,
                'message' => $e->getMessage(),
                'parent' => $parent,
                'entries' => [],
                'exclude_paths' => $excludePaths,
            ], 422);
        }
    }

    public function estimateSourceFootprint(Request $request, VedaCodeSource $codeSource, CodeSourceIndexer $indexer): JsonResponse
    {
        $data = $request->validate([
            'exclude_paths' => 'nullable|array|max:500',
            'exclude_paths.*' => 'string|max:512',
        ]);

        $excludePaths = $indexer->normalizeExcludePaths($data['exclude_paths'] ?? []);

        try {
            return response()->json($indexer->estimateIndexFootprintForSource($codeSource, $excludePaths));
        } catch (RuntimeException $e) {
            return response()->json(['message' => $e->getMessage()], 422);
        }
    }

    protected function localIndexingEnabled(): bool
    {
        return (bool) config('veda.code_index.allow_local_paths', true);
    }

    /**
     * @return array<int, array<string, mixed>>
     */
    protected function sourceList(): array
    {
        return VedaCodeSource::query()
            ->orderByDesc('updated_at')
            ->get()
            ->map(fn (VedaCodeSource $source) => $this->serializeSource($source))
            ->values()
            ->all();
    }

    /**
     * @return array<string, mixed>
     */
    protected function serializeSource(VedaCodeSource $source): array
    {
        return [
            'id' => $source->id,
            'name' => $source->name,
            'description' => $source->description,
            'provider' => $source->provider,
            'status' => $source->status,
            'last_indexed_at' => $source->last_indexed_at?->toIso8601String(),
            'metadata' => $source->metadata,
            'error_message' => $source->error_message,
            'local_absolute_path' => $source->local_absolute_path,
            'git_remote_url' => $source->git_remote_url,
            'git_branch' => $source->git_branch,
            'clone_relative_path' => $source->clone_relative_path,
            'indexing_progress' => $source->indexing_progress,
            'indexing_phase' => $source->indexing_phase,
            'index_cancel_requested' => $source->index_cancel_requested,
            'updated_at' => $source->updated_at?->toIso8601String(),
        ];
    }

    /**
     * @return array<string, mixed>
     */
    protected function browseDirectory(CodeSourceIndexer $indexer, string $real): array
    {
        $parent = dirname($real);
        $parentPath = null;
        if ($parent !== $real) {
            try {
                $parentPath = $indexer->assertAllowedLocalDirectory($parent);
            } catch (RuntimeException) {
                $parentPath = null;
            }
        }

        $entries = [];
        foreach (scandir($real) ?: [] as $name) {
            if ($name === '.' || $name === '..') {
                continue;
            }
            $full = $real.DIRECTORY_SEPARATOR.$name;
            if (! is_dir($full)) {
                continue;
            }
            $childReal = realpath($full);
            if ($childReal === false) {
                continue;
            }
            try {
                $indexer->assertAllowedLocalDirectory($childReal);
            } catch (RuntimeException) {
                continue;
            }
            $entries[] = [
                'name' => $name,
                'path' => $childReal,
            ];
        }

        usort($entries, fn (array $a, array $b): int => strcasecmp($a['name'], $b['name']));

        return [
            'current_path' => $real,
            'parent_path' => $parentPath,
            'entries' => $entries,
            'needs_anchor' => false,
        ];
    }
}
