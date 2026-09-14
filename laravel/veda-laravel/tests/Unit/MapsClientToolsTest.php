<?php

namespace Veda\Laravel\Tests\Unit;

use Illuminate\Contracts\Events\Dispatcher;
use ReflectionMethod;
use Veda\Laravel\Gateway\VedaResponsesGateway;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Tests\TestCase;

class MapsClientToolsTest extends TestCase
{
    protected function tearDown(): void
    {
        RequestContext::forget();

        parent::tearDown();
    }

    public function test_returns_empty_without_context_or_client_tools(): void
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));

        $this->assertSame([], $this->mapClientTools($gateway, []));

        RequestContext::bind([], false);

        $this->assertSame([], $this->mapClientTools($gateway, []));
    }

    public function test_maps_client_tools_to_chat_format(): void
    {
        RequestContext::bind([], false, clientTools: [
            [
                'name' => 'confirm_course_creation',
                'description' => 'Ask the user to confirm course creation',
                'parameters' => [
                    'type' => 'object',
                    'properties' => ['title' => ['type' => 'string']],
                    'required' => ['title'],
                ],
            ],
        ]);

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $mapped = $this->mapClientTools($gateway, []);

        $this->assertCount(1, $mapped);
        $this->assertSame('function', $mapped[0]['type']);
        $this->assertSame('confirm_course_creation', $mapped[0]['function']['name']);
        $this->assertSame('Ask the user to confirm course creation', $mapped[0]['function']['description']);
        $this->assertSame(['title'], $mapped[0]['function']['parameters']['required']);
    }

    public function test_maps_client_tools_to_responses_format(): void
    {
        RequestContext::bind([], false, clientTools: [
            ['name' => 'confirm_action', 'description' => 'Confirm', 'parameters' => ['type' => 'object']],
        ]);

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $mapped = $this->mapClientTools($gateway, [], 'responses');

        $this->assertCount(1, $mapped);
        $this->assertSame('function', $mapped[0]['type']);
        $this->assertSame('confirm_action', $mapped[0]['name']);
        $this->assertArrayNotHasKey('function', $mapped[0]);
    }

    public function test_skips_client_tools_colliding_with_mapped_backend_tools(): void
    {
        RequestContext::bind([], false, clientTools: [
            ['name' => 'search_knowledge_base', 'description' => 'Collision', 'parameters' => ['type' => 'object']],
            ['name' => 'confirm_action', 'description' => 'OK', 'parameters' => ['type' => 'object']],
        ]);

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $mapped = $this->mapClientTools($gateway, [
            ['type' => 'function', 'function' => ['name' => 'search_knowledge_base']],
        ]);

        $this->assertCount(1, $mapped);
        $this->assertSame('confirm_action', $mapped[0]['function']['name']);
    }

    public function test_adds_object_type_when_parameters_missing_it(): void
    {
        RequestContext::bind([], false, clientTools: [
            ['name' => 'confirm_action', 'description' => 'OK', 'parameters' => ['properties' => ['x' => ['type' => 'string']]]],
        ]);

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $mapped = $this->mapClientTools($gateway, []);

        $this->assertSame('object', $mapped[0]['function']['parameters']['type']);
        $this->assertSame(['x' => ['type' => 'string']], $mapped[0]['function']['parameters']['properties']);
    }

    public function test_maps_client_tools_to_anthropic_format(): void
    {
        RequestContext::bind([], false, clientTools: [
            [
                'name' => 'confirm_action',
                'description' => 'Confirm',
                'parameters' => [
                    'type' => 'object',
                    'properties' => ['ok' => ['type' => 'boolean']],
                ],
            ],
        ]);

        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $mapped = $this->mapClientTools($gateway, [], 'anthropic');

        $this->assertCount(1, $mapped);
        $this->assertSame('confirm_action', $mapped[0]['name']);
        $this->assertSame('Confirm', $mapped[0]['description']);
        $this->assertArrayHasKey('input_schema', $mapped[0]);
        $this->assertArrayNotHasKey('type', $mapped[0]);
        $this->assertArrayNotHasKey('parameters', $mapped[0]);
    }

    protected function mapClientTools(VedaResponsesGateway $gateway, array $mappedBackendTools, string $format = 'chat'): array
    {
        $method = new ReflectionMethod($gateway, 'mapClientTools');

        return $method->invoke($gateway, $mappedBackendTools, $format);
    }
}
