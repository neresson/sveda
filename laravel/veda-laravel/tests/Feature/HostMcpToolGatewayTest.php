<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Http\Client\Request;
use Illuminate\Support\Facades\Http;
use Laravel\Ai\Tools\Request as AiToolRequest;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;
use Veda\Laravel\Services\HostMcpCredentialStore;
use Veda\Laravel\Services\HostMcpToolGateway;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\HostMcpVedaTool;

class HostMcpToolGatewayTest extends TestCase
{
    public function test_returns_no_tools_without_visitor_credentials(): void
    {
        $this->assertSame([], app(HostMcpToolGateway::class)->tools());
    }

    public function test_loads_host_mcp_tools_for_the_current_visitor(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request));

        request()->attributes->set(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID, 'lms-20');
        app(HostMcpCredentialStore::class)->put(
            'lms-20',
            'http://lms.test/mcp/veda',
            'mcp-secret',
            3600,
        );

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);
        $this->assertInstanceOf(HostMcpVedaTool::class, $tools[0]);
        $this->assertSame('get_calendar_events', $tools[0]->name());
        $this->assertSame('calendar', $tools[0]->domain());

        $result = json_decode((string) $tools[0]->handle(new AiToolRequest(['limit' => 1])), true);
        $this->assertIsArray($result);
        $this->assertTrue((bool) ($result['success'] ?? false));
    }

    public function test_veda_agent_includes_host_mcp_tools(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request));

        request()->attributes->set(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID, 'lms-21');
        app(HostMcpCredentialStore::class)->put(
            'lms-21',
            'http://lms.test/mcp/veda',
            'mcp-secret',
            3600,
        );

        $names = [];
        foreach ((new VedaAgent)->tools() as $tool) {
            $names[] = $tool instanceof HostMcpVedaTool ? $tool->name() : $tool::class;
        }

        $this->assertContains('get_calendar_events', $names);
    }

    protected function fakeMcpResponse($request): mixed
    {
        if ($request->method() === 'DELETE') {
            return Http::response('', 200);
        }

        $payload = json_decode($request->body(), true);
        $method = is_array($payload) ? ($payload['method'] ?? '') : '';
        $id = is_array($payload) ? ($payload['id'] ?? 1) : 1;

        return match ($method) {
            'initialize' => Http::response([
                'jsonrpc' => '2.0',
                'id' => $id,
                'result' => [
                    'protocolVersion' => '2025-11-25',
                    'capabilities' => ['tools' => ['listChanged' => false]],
                    'serverInfo' => ['name' => 'lms', 'version' => '0.1.0'],
                ],
            ], 200, [
                'Content-Type' => 'application/json',
                'MCP-Session-Id' => 'sess-test',
            ]),
            'notifications/initialized' => Http::response('', 202),
            'tools/list' => Http::response([
                'jsonrpc' => '2.0',
                'id' => $id,
                'result' => [
                    'tools' => [[
                        'name' => 'get_calendar_events',
                        'title' => 'get_calendar_events',
                        'description' => 'Get calendar events',
                        'inputSchema' => [
                            'type' => 'object',
                            'properties' => [
                                'limit' => [
                                    'type' => 'integer',
                                    'description' => 'Maximum events',
                                ],
                            ],
                        ],
                        'annotations' => [],
                        '_meta' => [
                            'domain' => 'calendar',
                            'mode' => 'read',
                        ],
                    ]],
                ],
            ]),
            'tools/call' => Http::response([
                'jsonrpc' => '2.0',
                'id' => $id,
                'result' => [
                    'content' => [[
                        'type' => 'text',
                        'text' => '{"success":true,"data":{"events":[]}}',
                    ]],
                    'isError' => false,
                ],
            ]),
            default => Http::response('', 202),
        };
    }
}
