<?php

namespace Veda\Laravel\Tests\Unit;

use Illuminate\Contracts\Events\Dispatcher;
use ReflectionMethod;
use Veda\Laravel\Gateway\VedaResponsesGateway;
use Veda\Laravel\Services\ToolDeferralPolicy;
use Veda\Laravel\Tests\Fixtures\DummyReadTool;
use Veda\Laravel\Tests\TestCase;

class MapsActiveToolsTest extends TestCase
{
    protected function tearDown(): void
    {
        ToolDeferralPolicy::$deferralActive = false;

        parent::tearDown();
    }

    public function test_maps_all_tools_when_deferral_not_active(): void
    {
        config()->set('veda.tool_defer.enabled', true);
        ToolDeferralPolicy::$deferralActive = false;

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $tool = new DummyReadTool;

        $this->assertTrue($this->shouldMap($gateway, $tool, []));
    }

    public function test_maps_all_tools_when_deferral_disabled(): void
    {
        config()->set('veda.tool_defer.enabled', false);
        ToolDeferralPolicy::$deferralActive = true;

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $tool = new DummyReadTool;

        $this->assertTrue($this->shouldMap($gateway, $tool, []));
    }

    public function test_filters_inactive_tools_when_deferral_active(): void
    {
        config()->set('veda.tool_defer.enabled', true);
        ToolDeferralPolicy::$deferralActive = true;

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $tool = new DummyReadTool;

        $this->assertFalse($this->shouldMap($gateway, $tool, ['other_tool']));
        $this->assertTrue($this->shouldMap($gateway, $tool, ['dummy_read']));
    }

    protected function shouldMap(VedaResponsesGateway $gateway, object $tool, array $activeTools): bool
    {
        $method = new ReflectionMethod($gateway, 'shouldMapTool');

        return $method->invoke($gateway, $tool, $activeTools);
    }
}
