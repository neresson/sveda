<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Tests\TestCase;

class VedaAgentThinkingOptionsTest extends TestCase
{
    public function test_thinking_is_off_when_context_is_missing(): void
    {
        RequestContext::forget();

        $options = (new VedaAgent)->providerOptions('veda-responses');

        $this->assertSame('none', $options['reasoning']['effort'] ?? null);
    }

    public function test_thinking_is_off_when_client_sends_false(): void
    {
        RequestContext::bind([], false, thinkingEnabled: false);

        $options = (new VedaAgent)->providerOptions('veda-responses');

        $this->assertSame('none', $options['reasoning']['effort'] ?? null);
    }

    public function test_thinking_is_high_only_when_explicitly_enabled(): void
    {
        RequestContext::bind([], false, thinkingEnabled: true);

        $options = (new VedaAgent)->providerOptions('veda-responses');

        $this->assertSame('high', $options['reasoning']['effort'] ?? null);
    }
}
