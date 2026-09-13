<?php

namespace Veda\Laravel\Tests\Unit;

use Illuminate\Contracts\Auth\Authenticatable;
use Laravel\Ai\Tools\Request;
use Veda\Laravel\Contracts\ToolPermissionHook;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Services\ToolDeferralPolicy;
use Veda\Laravel\Tests\Fixtures\DummyReadTool;
use Veda\Laravel\Tests\Fixtures\DummyWriteTool;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\SearchAgentToolsTool;
use Veda\Laravel\Tools\ToolResolver;
use Veda\Laravel\VedaManager;

class ToolResolverTest extends TestCase
{
    protected function tearDown(): void
    {
        ToolDeferralPolicy::$dynamicallyActivatedTools = [];
        ToolDeferralPolicy::$deferralActive = false;
        RequestContext::forget();

        parent::tearDown();
    }

    public function test_resolves_registered_tools(): void
    {
        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        $resolved = app(ToolResolver::class)->resolve();

        $names = array_map(fn ($tool) => $tool->name(), $resolved['tools']);

        $this->assertContains('dummy_read', $names);
        $this->assertContains('dummy_write', $names);
        $this->assertFalse($resolved['deferred']);
    }

    public function test_permission_hook_filters_tools(): void
    {
        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        $manager->permissionHook(new class implements ToolPermissionHook
        {
            public function allows(?Authenticatable $user, string $toolName, ToolMode $mode): bool
            {
                return $mode === ToolMode::Read;
            }
        });

        $resolved = app(ToolResolver::class)->resolve();
        $names = array_map(fn ($tool) => $tool->name(), $resolved['tools']);

        $this->assertContains('dummy_read', $names);
        $this->assertNotContains('dummy_write', $names);
    }

    public function test_embed_mode_filters_write_tools(): void
    {
        config()->set('veda.embed.write_tools_enabled', false);

        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        $context = RequestContext::bind([], true);
        $resolved = app(ToolResolver::class)->resolve(null, $context);
        $names = array_map(fn ($tool) => $tool->name(), $resolved['tools']);

        $this->assertContains('dummy_read', $names);
        $this->assertNotContains('dummy_write', $names);
    }

    public function test_embed_write_filter_can_be_overridden_via_subclass(): void
    {
        config()->set('veda.embed.write_tools_enabled', false);

        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        app()->bind(ToolResolver::class, fn ($app) => new class($app->make(VedaManager::class), $app->make(ToolDeferralPolicy::class)) extends ToolResolver
        {
            protected function embedWriteToolsEnabled(?Authenticatable $user): bool
            {
                return true;
            }
        });

        $context = RequestContext::bind([], true);
        $resolved = app(ToolResolver::class)->resolve(null, $context);
        $names = array_map(fn ($tool) => $tool->name(), $resolved['tools']);

        $this->assertContains('dummy_read', $names);
        $this->assertContains('dummy_write', $names);
    }

    public function test_deferral_adds_search_tool_when_pool_is_large(): void
    {
        config()->set('veda.tool_defer.enabled', true);
        config()->set('veda.tool_defer.min_pool', 2);
        config()->set('veda.tool_defer.always_loaded', ['dummy_read']);

        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        $resolved = app(ToolResolver::class)->resolve();

        $this->assertTrue($resolved['deferred']);

        $names = array_map(fn ($tool) => $tool->name(), $resolved['tools']);

        $this->assertContains('dummy_read', $names);
        $this->assertNotContains('dummy_write', $names);
        $this->assertContains(ToolDeferralPolicy::SEARCH_TOOL_NAME, $names);

        $searchTool = collect($resolved['tools'])->first(
            fn ($tool) => $tool->name() === ToolDeferralPolicy::SEARCH_TOOL_NAME
        );

        $this->assertInstanceOf(SearchAgentToolsTool::class, $searchTool);
    }

    public function test_deferral_disabled_returns_all_tools(): void
    {
        config()->set('veda.tool_defer.enabled', false);
        config()->set('veda.tool_defer.min_pool', 1);

        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        $resolved = app(ToolResolver::class)->resolve();

        $this->assertFalse($resolved['deferred']);
        $this->assertCount(2, $resolved['tools']);
        $this->assertFalse(ToolDeferralPolicy::$deferralActive);
    }

    public function test_resolve_marks_deferral_active_for_gateway_mapping(): void
    {
        config()->set('veda.tool_defer.enabled', true);
        config()->set('veda.tool_defer.min_pool', 2);
        config()->set('veda.tool_defer.always_loaded', ['dummy_read']);

        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        app(ToolResolver::class)->resolve();

        $this->assertTrue(ToolDeferralPolicy::$deferralActive);
    }

    public function test_eager_resolve_does_not_touch_deferral_state(): void
    {
        config()->set('veda.tool_defer.enabled', true);
        config()->set('veda.tool_defer.min_pool', 2);
        config()->set('veda.tool_defer.always_loaded', ['dummy_read']);

        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        ToolDeferralPolicy::$deferralActive = true;

        $resolved = app(ToolResolver::class)->resolve(eagerLoadAll: true);

        $this->assertFalse($resolved['deferred']);
        $this->assertCount(2, $resolved['tools']);
        $this->assertTrue(ToolDeferralPolicy::$deferralActive);
    }

    public function test_search_tool_activates_deferred_tools(): void
    {
        config()->set('veda.tool_defer.enabled', true);
        config()->set('veda.tool_defer.min_pool', 2);
        config()->set('veda.tool_defer.always_loaded', []);
        config()->set('veda.tool_catalog.embeddings_enabled', false);

        $manager = app(VedaManager::class);
        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyWriteTool::class);

        $resolved = app(ToolResolver::class)->resolve();

        $searchTool = collect($resolved['tools'])->first(
            fn ($tool) => $tool->name() === ToolDeferralPolicy::SEARCH_TOOL_NAME
        );

        $this->assertNotNull($searchTool);

        $result = $searchTool->handle(new Request(['query' => 'dummy write']));
        $payload = json_decode((string) $result, true);

        $this->assertTrue($payload['success']);
        $this->assertContains('dummy_write', $payload['data']['activated_tools']);
        $this->assertContains('dummy_write', ToolDeferralPolicy::$dynamicallyActivatedTools);
    }
}
