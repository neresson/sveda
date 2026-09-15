<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Laravel\Ai\Tools\Request as AiToolRequest;
use Veda\Laravel\CodeIndex\CodeIndexSourcesContext;
use Veda\Laravel\Models\VedaCodeIndexChunk;
use Veda\Laravel\Models\VedaCodeSource;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\ListCodeSourcesTool;
use Veda\Laravel\Tools\SearchCodeTool;

class CodeIndexAdminTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.admin.api_key', 'veda-admin-secret');
    }

    protected function setUp(): void
    {
        parent::setUp();

        $this->withoutMiddleware(ValidateCsrfToken::class);
    }

    public function test_guest_cannot_list_code_sources(): void
    {
        $this->getJson('/veda/admin/code-index/sources')->assertUnauthorized();
    }

    public function test_admin_can_create_index_and_search_a_local_folder(): void
    {
        $root = $this->makeFixtureTree();

        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->getJson('/veda/admin/sources')
            ->assertOk()
            ->assertJsonPath('page', 'sources');

        $this->assertStringContainsString(
            '/veda/admin/code-index/sources',
            (string) $this->getJson('/veda/admin/sources')->json('codeIndex.sources'),
        );

        $created = $this->postJson('/veda/admin/code-index/sources', [
            'name' => 'Veda fixture',
            'provider' => 'local',
            'description' => 'Local fixture for search',
            'local_absolute_path' => $root,
        ])->assertCreated();

        $sourceId = (int) $created->json('source.id');
        $this->assertGreaterThan(0, $sourceId);

        $this->patchJson('/veda/admin/code-index/sources/'.$sourceId, [
            'name' => 'Veda fixture',
            'description' => 'Local fixture for search',
            'exclude_paths' => [],
            'start_indexing' => true,
        ])->assertOk();

        $source = VedaCodeSource::query()->find($sourceId);
        $this->assertNotNull($source);
        $this->assertSame('ready', $source->status);
        $this->assertGreaterThan(0, VedaCodeIndexChunk::query()->where('code_source_id', $sourceId)->count());

        $listed = json_decode((string) (new ListCodeSourcesTool)->handle(new AiToolRequest([])), true);
        $this->assertTrue((bool) ($listed['success'] ?? false));
        $this->assertSame($sourceId, $listed['data']['sources'][0]['id'] ?? null);

        $search = json_decode((string) (new SearchCodeTool)->handle(new AiToolRequest([
            'query' => 'vedacode',
            'source_id' => $sourceId,
        ])), true);
        $this->assertTrue((bool) ($search['success'] ?? false));
        $this->assertGreaterThan(0, (int) ($search['data']['span_count'] ?? 0));

        $section = app(CodeIndexSourcesContext::class)->promptSection();
        $this->assertStringContainsString('source_id '.$sourceId, $section);
        $this->assertStringContainsString('Veda fixture', $section);
    }

    public function test_local_browse_requires_anchor_then_lists_children(): void
    {
        $root = $this->makeFixtureTree();

        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->getJson('/veda/admin/code-index/local-browse')
            ->assertOk()
            ->assertJsonPath('needs_anchor', true);

        $this->getJson('/veda/admin/code-index/local-browse?path='.urlencode($root))
            ->assertOk()
            ->assertJsonPath('needs_anchor', false)
            ->assertJsonPath('current_path', realpath($root));
    }

    protected function makeFixtureTree(): string
    {
        $root = sys_get_temp_dir().'/veda-code-index-admin-'.bin2hex(random_bytes(6));
        mkdir($root.'/src', 0777, true);
        file_put_contents($root.'/src/Hello.php', "<?php\n\nclass Hello {\n    public function world(): string\n    {\n        return 'vedacode';\n    }\n}\n");

        return realpath($root) ?: $root;
    }
}
