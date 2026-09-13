<?php

namespace Veda\Laravel\Tests\Unit;

use Laravel\Ai\Tools\Request;
use Veda\Laravel\Tests\Fixtures\DummyReadTool;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\ToolArgumentNormalizer;
use Veda\Laravel\VedaManager;

class VedaToolTest extends TestCase
{
    public function test_handle_returns_normalized_success_payload(): void
    {
        $tool = new DummyReadTool;

        $result = json_decode((string) $tool->handle(new Request(['query' => 'abc'])), true);

        $this->assertTrue($result['success']);
        $this->assertSame(['query' => 'abc'], $result['data']);
    }

    public function test_handle_catches_exceptions(): void
    {
        $tool = new class extends DummyReadTool
        {
            protected function execute(array $arguments): array|string
            {
                throw new \RuntimeException('boom');
            }
        };

        $result = json_decode((string) $tool->handle(new Request(['query' => 'abc'])), true);

        $this->assertFalse($result['success']);
        $this->assertSame('boom', $result['error']);
    }

    public function test_result_enricher_hook_is_applied(): void
    {
        app(VedaManager::class)->toolResultEnricher(
            fn (string $tool, array $arguments, array $result) => $result + ['enriched' => true]
        );

        $tool = new DummyReadTool;

        $result = json_decode((string) $tool->handle(new Request(['query' => 'abc'])), true);

        $this->assertTrue($result['enriched']);
    }

    public function test_handle_resolves_argument_normalizer_from_container(): void
    {
        app()->bind(ToolArgumentNormalizer::class, fn () => new class extends ToolArgumentNormalizer
        {
            public function normalize(string $toolName, array $arguments): array
            {
                $normalized = parent::normalize($toolName, $arguments);
                $normalized['query'] = 'from-container';

                return $normalized;
            }
        });

        $tool = new DummyReadTool;

        $result = json_decode((string) $tool->handle(new Request(['query' => 'abc'])), true);

        $this->assertTrue($result['success']);
        $this->assertSame(['query' => 'from-container'], $result['data']);
    }
}
