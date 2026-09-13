<?php

namespace Veda\Laravel\Tests\Unit;

use Illuminate\Contracts\Auth\Authenticatable;
use Veda\Laravel\Contracts\TokenPolicy;
use Veda\Laravel\Contracts\ToolPermissionHook;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Support\AllowAllToolsPermissionHook;
use Veda\Laravel\Support\NullTokenPolicy;
use Veda\Laravel\Tests\Fixtures\DummyReadTool;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\ClosureTool;
use Veda\Laravel\VedaManager;

class VedaManagerTest extends TestCase
{
    public function test_registers_closure_tool(): void
    {
        $manager = app(VedaManager::class);

        $manager->tool(
            'get_weather',
            'Get weather by city.',
            ['properties' => ['city' => ['type' => 'string']], 'required' => ['city']],
            fn (array $args) => ['success' => true, 'data' => ['city' => $args['city'] ?? null]],
            ToolMode::Read,
            'weather',
        );

        $definitions = $manager->toolDefinitions();

        $this->assertArrayHasKey('get_weather', $definitions);
        $this->assertSame('Get weather by city.', $definitions['get_weather']['description']);
        $this->assertSame(ToolMode::Read, $definitions['get_weather']['mode']);
        $this->assertSame('weather', $definitions['get_weather']['domain']);
    }

    public function test_registers_tool_class(): void
    {
        $manager = app(VedaManager::class);

        $manager->toolClass(DummyReadTool::class);
        $manager->toolClass(DummyReadTool::class);

        $this->assertSame([DummyReadTool::class], $manager->toolClasses());
    }

    public function test_tool_instances_include_classes_and_closures(): void
    {
        $manager = app(VedaManager::class);

        $manager->toolClass(DummyReadTool::class);
        $manager->tool('get_time', 'Get current time.', [], fn () => '12:00');

        $instances = $manager->toolInstances();

        $this->assertCount(2, $instances);
        $this->assertInstanceOf(DummyReadTool::class, $instances[0]);
        $this->assertInstanceOf(ClosureTool::class, $instances[1]);
        $this->assertSame('get_time', $instances[1]->name());
    }

    public function test_context_providers_are_collected(): void
    {
        $manager = app(VedaManager::class);

        $manager->contextProvider(fn () => 'section A');
        $manager->contextProvider(fn () => 'section B');

        $this->assertCount(2, $manager->contextProviders());
    }

    public function test_default_token_policy_is_null_implementation(): void
    {
        $manager = app(VedaManager::class);

        $this->assertInstanceOf(NullTokenPolicy::class, $manager->getTokenPolicy());
    }

    public function test_custom_token_policy_can_be_set(): void
    {
        $manager = app(VedaManager::class);
        $policy = new class implements TokenPolicy
        {
            public function assertRequestAllowed(?Authenticatable $user, int $estimatedTokens): void {}

            public function recordUsage(?Authenticatable $user, string $provider, string $model, int $promptTokens, int $completionTokens): void {}
        };

        $manager->tokenPolicy($policy);

        $this->assertSame($policy, $manager->getTokenPolicy());
    }

    public function test_default_permission_hook_allows_all(): void
    {
        $manager = app(VedaManager::class);

        $hook = $manager->getPermissionHook();

        $this->assertInstanceOf(AllowAllToolsPermissionHook::class, $hook);
        $this->assertTrue($hook->allows(null, 'any_tool', ToolMode::Write));
    }

    public function test_custom_permission_hook_can_be_set(): void
    {
        $manager = app(VedaManager::class);
        $hook = new class implements ToolPermissionHook
        {
            public function allows(?Authenticatable $user, string $toolName, ToolMode $mode): bool
            {
                return true;
            }
        };

        $manager->permissionHook($hook);

        $this->assertSame($hook, $manager->getPermissionHook());
    }

    public function test_tool_alias_normalization(): void
    {
        $manager = app(VedaManager::class);

        $manager->toolAlias('legacy_name', 'canonical_name');

        $this->assertSame('canonical_name', $manager->normalizeToolName('legacy_name'));
        $this->assertSame('other', $manager->normalizeToolName('other'));
    }

    public function test_custom_tool_name_normalizer_is_applied_after_aliases(): void
    {
        $manager = app(VedaManager::class);

        $manager->toolAlias('legacy_name', 'canonical_name');
        $manager->toolNameNormalizer(fn (string $name) => str_starts_with($name, 'teacher_') ? substr($name, 8) : $name);

        $this->assertSame('list_courses', $manager->normalizeToolName('teacher_list_courses'));
        $this->assertSame('canonical_name', $manager->normalizeToolName('legacy_name'));
        $this->assertSame('other', $manager->normalizeToolName('other'));
    }

    public function test_response_guard_passthrough_without_registration(): void
    {
        $manager = app(VedaManager::class);

        $this->assertSame('raw text', $manager->applyResponseGuard('raw text', ['prompt' => 'q']));
    }

    public function test_response_guard_is_applied(): void
    {
        $manager = app(VedaManager::class);

        $manager->responseGuard(fn (string $text, array $context) => $text.' [guarded:'.$context['prompt'].']');

        $this->assertSame('raw text [guarded:q]', $manager->applyResponseGuard('raw text', ['prompt' => 'q']));
    }

    public function test_response_guard_falls_back_on_non_string_return(): void
    {
        $manager = app(VedaManager::class);

        $manager->responseGuard(fn () => null);

        $this->assertSame('raw text', $manager->applyResponseGuard('raw text'));
    }

    public function test_global_context_provider_defaults_to_null(): void
    {
        $manager = app(VedaManager::class);

        $this->assertNull($manager->resolveGlobalContextBlock(null));
    }

    public function test_global_context_provider_is_applied(): void
    {
        $manager = app(VedaManager::class);

        $manager->globalContextProvider(fn ($user, array $pageContext) => 'block:'.$pageContext['origin']);

        $this->assertSame('block:lms', $manager->resolveGlobalContextBlock(null, ['origin' => 'lms']));
    }

    public function test_global_context_provider_falls_back_on_empty_return(): void
    {
        $manager = app(VedaManager::class);

        $manager->globalContextProvider(fn () => '   ');

        $this->assertNull($manager->resolveGlobalContextBlock(null));
    }

    public function test_flush_clears_all_registrations(): void
    {
        $manager = app(VedaManager::class);

        $manager->tool('get_time', 'x', [], fn () => 'ok');
        $manager->toolClass(DummyReadTool::class);
        $manager->contextProvider(fn () => 'x');
        $manager->responseGuard(fn (string $text) => $text);
        $manager->globalContextProvider(fn () => 'x');

        $manager->flush();

        $this->assertSame([], $manager->toolDefinitions());
        $this->assertSame([], $manager->toolClasses());
        $this->assertSame([], $manager->contextProviders());
        $this->assertSame('raw', $manager->applyResponseGuard('raw'));
        $this->assertNull($manager->resolveGlobalContextBlock(null));
    }
}
