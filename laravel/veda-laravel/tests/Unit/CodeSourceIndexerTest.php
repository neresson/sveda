<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\CodeIndex\CodeIndexEmbeddingService;
use Veda\Laravel\CodeIndex\CodeSourceIndexer;
use Veda\Laravel\Models\VedaCodeIndexChunk;
use Veda\Laravel\Models\VedaCodeSource;
use Veda\Laravel\Tests\TestCase;

class CodeSourceIndexerTest extends TestCase
{
    public function test_indexes_local_folder_and_skips_env_and_vendor(): void
    {
        $root = $this->makeFixtureTree();
        $source = VedaCodeSource::query()->create([
            'name' => 'Fixture',
            'description' => 'Test source',
            'provider' => 'local',
            'local_absolute_path' => $root,
            'status' => 'pending',
            'metadata' => ['exclude_paths' => [], 'workspace_ready' => true],
        ]);

        app(CodeSourceIndexer::class)->index($source->fresh());

        $source->refresh();
        $this->assertSame('ready', $source->status);
        $this->assertGreaterThan(0, (int) ($source->metadata['chunks_total'] ?? 0));

        $paths = VedaCodeIndexChunk::query()
            ->where('code_source_id', $source->id)
            ->pluck('path')
            ->all();

        $this->assertContains('src/Hello.php', $paths);
        $this->assertFalse(collect($paths)->contains(fn (string $path): bool => str_contains($path, 'vendor/')));
        $this->assertFalse(collect($paths)->contains(fn (string $path): bool => str_ends_with($path, '.env')));
    }

    public function test_honors_user_exclude_paths(): void
    {
        $root = $this->makeFixtureTree();
        mkdir($root.'/skip', 0777, true);
        file_put_contents($root.'/skip/Secret.php', "<?php echo 'secret';\n");

        $source = VedaCodeSource::query()->create([
            'name' => 'Fixture',
            'provider' => 'local',
            'local_absolute_path' => $root,
            'status' => 'pending',
            'metadata' => ['exclude_paths' => ['skip'], 'workspace_ready' => true],
        ]);

        app(CodeSourceIndexer::class)->index($source->fresh());

        $source->refresh();
        $this->assertSame('ready', $source->status);
        $this->assertSame(['skip'], $source->metadata['exclude_paths'] ?? null);

        $paths = VedaCodeIndexChunk::query()
            ->where('code_source_id', $source->id)
            ->pluck('path')
            ->all();

        $this->assertContains('src/Hello.php', $paths);
        $this->assertFalse(collect($paths)->contains(fn (string $path): bool => str_starts_with($path, 'skip/')));
    }

    public function test_ready_when_embeddings_are_unavailable(): void
    {
        $this->mock(CodeIndexEmbeddingService::class, function ($mock): void {
            $mock->shouldReceive('isConfigured')->andReturn(true);
            $mock->shouldReceive('isAvailable')->andReturn(false);
        });

        $root = $this->makeFixtureTree();
        $source = VedaCodeSource::query()->create([
            'name' => 'Fixture',
            'provider' => 'local',
            'local_absolute_path' => $root,
            'status' => 'pending',
            'metadata' => ['exclude_paths' => [], 'workspace_ready' => true],
        ]);

        app(CodeSourceIndexer::class)->index($source->fresh());

        $source->refresh();
        $this->assertSame('ready', $source->status);
        $this->assertSame(0, (int) ($source->metadata['chunks_embedded'] ?? -1));
        $this->assertGreaterThan(0, (int) ($source->metadata['chunks_total'] ?? 0));
    }

    public function test_rejects_local_path_when_disabled(): void
    {
        config()->set('veda.code_index.allow_local_paths', false);

        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('local_paths_disabled_set_allow_local_paths_true');

        app(CodeSourceIndexer::class)->assertAllowedLocalDirectory(sys_get_temp_dir());
    }

    protected function makeFixtureTree(): string
    {
        $root = sys_get_temp_dir().'/veda-code-index-'.bin2hex(random_bytes(6));
        mkdir($root.'/src', 0777, true);
        mkdir($root.'/vendor/pkg', 0777, true);
        file_put_contents($root.'/src/Hello.php', "<?php\n\nclass Hello {\n    public function world(): string\n    {\n        return 'vedacode';\n    }\n}\n");
        file_put_contents($root.'/vendor/pkg/Skip.php', "<?php echo 'skip';\n");
        file_put_contents($root.'/.env', "SECRET=1\n");

        return $root;
    }
}
