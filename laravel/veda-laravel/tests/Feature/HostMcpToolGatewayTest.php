<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Http\Client\Request;
use Illuminate\Support\Facades\Http;
use Laravel\Ai\Tools\Request as AiToolRequest;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;
use Veda\Laravel\Services\HostMcpCredentialStore;
use Veda\Laravel\Services\HostMcpToolGateway;
use Veda\Laravel\Services\McpCatalog;
use Veda\Laravel\Services\VedaSettingsRepository;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\HostMcpVedaTool;

class HostMcpToolGatewayTest extends TestCase
{
    public function test_returns_no_tools_without_visitor_credentials(): void
    {
        $this->assertSame([], app(HostMcpToolGateway::class)->tools());
    }

    public function test_ignores_host_credentials_unless_lms_is_in_the_catalog(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request));

        request()->attributes->set(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID, 'lms-19');
        app(HostMcpCredentialStore::class)->put(
            'lms-19',
            'http://lms.test/mcp/veda',
            'mcp-secret',
            3600,
        );

        $this->assertSame([], app(HostMcpToolGateway::class)->tools());
    }

    public function test_loads_host_mcp_tools_for_the_current_visitor(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request));

        $this->putLmsCatalog();
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

        $this->putLmsCatalog();
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

    public function test_loads_configured_mcp_servers_without_visitor_credentials(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request, 'search_docs'));

        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'docs' => [
                        'url' => 'http://docs.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer docs-secret',
                        ],
                    ],
                ],
            ],
        ]);

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);
        $this->assertInstanceOf(HostMcpVedaTool::class, $tools[0]);
        $this->assertSame('docs__search_docs', $tools[0]->name());

        $result = json_decode((string) $tools[0]->handle(new AiToolRequest(['query' => 'onboarding'])), true);
        $this->assertIsArray($result);
        $this->assertTrue((bool) ($result['success'] ?? false));
    }

    public function test_skips_disabled_configured_mcp_servers(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request));

        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'docs' => [
                        'url' => 'http://docs.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer docs-secret',
                        ],
                        'disabled' => true,
                    ],
                ],
            ],
        ]);

        $this->assertSame([], app(HostMcpToolGateway::class)->tools());
    }

    public function test_merges_session_host_tools_with_configured_mcp_servers(): void
    {
        Http::preventStrayRequests();
        Http::fake(function (Request $request) {
            $tool = str_contains((string) $request->url(), 'docs.test')
                ? 'search_docs'
                : 'get_calendar_events';

            return $this->fakeMcpResponse($request, $tool);
        });

        request()->attributes->set(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID, 'lms-22');
        app(HostMcpCredentialStore::class)->put(
            'lms-22',
            'http://lms.test/mcp/veda',
            'mcp-secret',
            3600,
        );
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'lms' => [
                        'url' => 'http://lms.test/mcp/veda',
                    ],
                    'docs' => [
                        'url' => 'http://docs.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer docs-secret',
                        ],
                    ],
                ],
            ],
        ]);

        $names = array_map(
            fn ($tool): string => $tool->name(),
            app(HostMcpToolGateway::class)->tools(),
        );

        $this->assertContains('get_calendar_events', $names);
        $this->assertContains('docs__search_docs', $names);
    }

    public function test_keeps_session_host_tools_when_a_configured_server_fails(): void
    {
        Http::preventStrayRequests();
        Http::fake(function (Request $request) {
            if (str_contains((string) $request->url(), 'broken.test')) {
                return Http::response('', 500);
            }

            return $this->fakeMcpResponse($request);
        });

        request()->attributes->set(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID, 'lms-23');
        app(HostMcpCredentialStore::class)->put(
            'lms-23',
            'http://lms.test/mcp/veda',
            'mcp-secret',
            3600,
        );
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'lms' => [
                        'url' => 'http://lms.test/mcp/veda',
                    ],
                    'broken' => [
                        'url' => 'http://broken.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer broken-secret',
                        ],
                    ],
                ],
            ],
        ]);

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);
        $this->assertSame('get_calendar_events', $tools[0]->name());
    }

    protected function fakeMcpResponse($request, string $toolName = 'get_calendar_events'): mixed
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
                        'name' => $toolName,
                        'title' => $toolName,
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

    public function test_interpolates_env_variables_in_http_headers(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request, 'search_docs'));

        $this->app->instance(McpCatalog::class, new McpCatalog(
            env: fn (string $name): ?string => $name === 'DOCS_MCP_TOKEN' ? 'docs-secret' : null,
        ));

        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'docs' => [
                        'url' => 'http://docs.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer ${env:DOCS_MCP_TOKEN}',
                        ],
                    ],
                ],
            ],
        ]);

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);
        $this->assertSame('docs__search_docs', $tools[0]->name());

        Http::assertSent(fn (Request $request): bool => $request->hasHeader('Authorization', 'Bearer docs-secret'));
    }

    public function test_loads_legacy_mcp_servers_array(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request, 'search_docs'));

        app(VedaSettingsRepository::class)->update([
            'mcp_servers' => [[
                'id' => 'docs',
                'label' => 'Docs',
                'url' => 'http://docs.test/mcp',
                'token' => 'docs-secret',
                'enabled' => true,
            ]],
        ]);

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);
        $this->assertSame('docs__search_docs', $tools[0]->name());
    }

    public function test_loads_stdio_mcp_tools_with_env(): void
    {
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'local' => $this->stdioServer([
                        'env' => [
                            'VEDA_STDIO_TOKEN' => 'from-env',
                        ],
                    ]),
                ],
            ],
        ]);

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);
        $this->assertSame('local__echo_env', $tools[0]->name());
        $this->assertSame('testing', $tools[0]->domain());

        $result = json_decode((string) $tools[0]->handle(new AiToolRequest([])), true);
        $this->assertIsArray($result);
        $this->assertTrue((bool) ($result['success'] ?? false));
        $this->assertSame('from-env', $result['data']['token'] ?? null);
    }

    public function test_loads_stdio_env_file_and_lets_env_override_it(): void
    {
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'local' => $this->stdioServer([
                        'envFile' => $this->stdioEnvFile(),
                        'env' => [
                            'VEDA_STDIO_TOKEN' => 'from-env',
                        ],
                    ]),
                ],
            ],
        ]);

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);

        $result = json_decode((string) $tools[0]->handle(new AiToolRequest([])), true);
        $this->assertIsArray($result);
        $this->assertSame('from-env', $result['data']['token'] ?? null);
        $this->assertSame('yes', $result['data']['from_file'] ?? null);
    }

    public function test_loads_stdio_env_file_when_env_is_absent(): void
    {
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'local' => $this->stdioServer([
                        'envFile' => $this->stdioEnvFile(),
                    ]),
                ],
            ],
        ]);

        $result = json_decode((string) app(HostMcpToolGateway::class)->tools()[0]->handle(new AiToolRequest([])), true);
        $this->assertIsArray($result);
        $this->assertSame('from-file', $result['data']['token'] ?? null);
        $this->assertSame('yes', $result['data']['from_file'] ?? null);
    }

    public function test_skips_disabled_stdio_mcp_servers(): void
    {
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'local' => $this->stdioServer([
                        'disabled' => true,
                    ]),
                ],
            ],
        ]);

        $this->assertSame([], app(HostMcpToolGateway::class)->tools());
    }

    public function test_keeps_http_tools_when_stdio_server_fails(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request, 'search_docs'));

        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'docs' => [
                        'url' => 'http://docs.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer docs-secret',
                        ],
                    ],
                    'local' => [
                        'command' => 'veda-stdio-missing-'.uniqid(),
                    ],
                ],
            ],
        ]);

        $tools = app(HostMcpToolGateway::class)->tools();
        $this->assertCount(1, $tools);
        $this->assertSame('docs__search_docs', $tools[0]->name());
    }

    public function test_merges_stdio_tools_with_http_servers(): void
    {
        Http::preventStrayRequests();
        Http::fake(fn (Request $request) => $this->fakeMcpResponse($request, 'search_docs'));

        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'docs' => [
                        'url' => 'http://docs.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer docs-secret',
                        ],
                    ],
                    'local' => $this->stdioServer([
                        'env' => [
                            'VEDA_STDIO_TOKEN' => 'from-env',
                        ],
                    ]),
                ],
            ],
        ]);

        $names = array_map(
            fn ($tool): string => $tool->name(),
            app(HostMcpToolGateway::class)->tools(),
        );

        $this->assertContains('docs__search_docs', $names);
        $this->assertContains('local__echo_env', $names);
    }

    protected function putLmsCatalog(string $url = 'http://lms.test/mcp/veda'): void
    {
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'lms' => [
                        'url' => $url,
                    ],
                ],
            ],
        ]);
    }

    /**
     * @param  array<string, mixed>  $extra
     * @return array<string, mixed>
     */
    protected function stdioServer(array $extra = []): array
    {
        $script = realpath(__DIR__.'/../Fixtures/stdio-mcp-server.php');
        $this->assertNotFalse($script);

        return [
            'command' => PHP_BINARY,
            'args' => [$script],
            ...$extra,
        ];
    }

    protected function stdioEnvFile(): string
    {
        $path = realpath(__DIR__.'/../Fixtures/stdio.env');
        $this->assertNotFalse($path);

        return $path;
    }
}
