<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Http\Client\Request;
use Illuminate\Support\Facades\Http;
use Laravel\Ai\Tools\Request as AiToolRequest;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Http\Controllers\VedaAdminChatSessionController;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;
use Veda\Laravel\Services\HostMcpToolGateway;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Services\VedaSettingsRepository;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\ManageMcpCatalogTool;

class ManageMcpCatalogToolTest extends TestCase
{
    public function test_connects_http_mcp_server_and_returns_prefixed_tools(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request, 'search_products'));

        $result = json_decode((string) (new ManageMcpCatalogTool)->handle(new AiToolRequest([
            'action' => 'connect',
            'url' => 'https://mcp.vkusvill.ru/mcp',
        ])), true);

        $this->assertTrue((bool) ($result['success'] ?? false));
        $this->assertSame('vkusvill', $result['data']['id'] ?? null);
        $this->assertSame(['vkusvill__search_products'], $result['data']['tools'] ?? null);
        $this->assertTrue((bool) ($result['data']['tools_ready'] ?? false));

        $document = app(VedaSettingsRepository::class)->document();
        $this->assertSame('https://mcp.vkusvill.ru/mcp', $document['mcp']['mcpServers']['vkusvill']['url'] ?? null);
    }

    public function test_lists_and_disconnects_catalog_servers(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request, 'search_products'));

        (new ManageMcpCatalogTool)->handle(new AiToolRequest([
            'action' => 'connect',
            'url' => 'https://mcp.vkusvill.ru/mcp',
        ]));

        $listed = json_decode((string) (new ManageMcpCatalogTool)->handle(new AiToolRequest([
            'action' => 'list',
        ])), true);

        $this->assertTrue((bool) ($listed['success'] ?? false));
        $this->assertSame('vkusvill', $listed['data']['servers'][0]['id'] ?? null);
        $this->assertFalse((bool) ($listed['data']['servers'][0]['disabled'] ?? true));

        $disconnected = json_decode((string) (new ManageMcpCatalogTool)->handle(new AiToolRequest([
            'action' => 'disconnect',
            'id' => 'vkusvill',
        ])), true);

        $this->assertTrue((bool) ($disconnected['success'] ?? false));
        $this->assertTrue((bool) (app(VedaSettingsRepository::class)->document()['mcp']['mcpServers']['vkusvill']['disabled'] ?? false));
        $this->assertSame([], app(HostMcpToolGateway::class)->tools());
    }

    public function test_rejects_reserved_lms_id(): void
    {
        $result = json_decode((string) (new ManageMcpCatalogTool)->handle(new AiToolRequest([
            'action' => 'connect',
            'id' => 'lms',
            'url' => 'https://mcp.vkusvill.ru/mcp',
        ])), true);

        $this->assertFalse((bool) ($result['success'] ?? true));
    }

    public function test_admin_embed_session_keeps_the_catalog_tool(): void
    {
        config()->set('veda.embed.write_tools_enabled', false);
        request()->attributes->set(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID, VedaAdminChatSessionController::VISITOR_ID);
        RequestContext::bind([], true);

        $names = array_map(
            fn ($tool): string => $tool->name(),
            (new VedaAgent)->tools(),
        );

        $this->assertContains(ManageMcpCatalogTool::NAME, $names);
    }

    public function test_embed_visitor_does_not_get_the_catalog_tool(): void
    {
        config()->set('veda.embed.write_tools_enabled', false);
        request()->attributes->set(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID, 'lms-visitor');
        RequestContext::bind([], true);

        $names = array_map(
            fn ($tool): string => $tool->name(),
            (new VedaAgent)->tools(),
        );

        $this->assertNotContains(ManageMcpCatalogTool::NAME, $names);
    }

    protected function fakeMcpResponse(Request $request, string $toolName): mixed
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
                    'serverInfo' => ['name' => 'vkusvill', 'version' => '0.1.0'],
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
                        'name' => $toolName,
                        'title' => $toolName,
                        'description' => 'Search products',
                        'inputSchema' => [
                            'type' => 'object',
                            'properties' => (object) [],
                        ],
                        'annotations' => [],
                        '_meta' => [
                            'domain' => 'other',
                            'mode' => 'read',
                        ],
                    ]],
                ],
            ]),
            default => Http::response('', 202),
        };
    }
}
