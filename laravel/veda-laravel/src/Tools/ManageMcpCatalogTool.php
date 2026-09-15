<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Laravel\Ai\Tools\ToolNameResolver;
use Stringable;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Http\Controllers\VedaAdminChatSessionController;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;
use Veda\Laravel\Services\HostMcpToolGateway;
use Veda\Laravel\Services\McpCatalog;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Services\ToolDeferralPolicy;
use Veda\Laravel\Services\VedaSettingsRepository;

class ManageMcpCatalogTool extends VedaTool
{
    public const NAME = 'manage_mcp_catalog';

    public function name(): string
    {
        return self::NAME;
    }

    public function description(): Stringable|string
    {
        return 'Connect, list, or disconnect HTTP MCP servers in this Veda instance catalog. Use this when the user gives an MCP URL. You cannot browse the public internet yourself. After connecting, call the returned MCP tools in the next step.';
    }

    public function mode(): ToolMode
    {
        return ToolMode::Write;
    }

    public function domain(): string
    {
        return 'mcp';
    }

    public function availableInSession(): bool
    {
        $visitorId = request()->attributes->get(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID);
        if (is_string($visitorId) && $visitorId === VedaAdminChatSessionController::VISITOR_ID) {
            return true;
        }

        return RequestContext::current()?->isEmbedMode !== true;
    }

    public function allowedDuringEmbedWriteFilter(): bool
    {
        return true;
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'action' => $schema->string()->description('One of: connect, disconnect, list.')->required(),
            'url' => $schema->string()->description('HTTP MCP endpoint, e.g. https://mcp.example.com/mcp. Required for connect.'),
            'id' => $schema->string()->description('Optional catalog id. Derived from the host when omitted.'),
            'authorization' => $schema->string()->description('Optional Authorization header value. A token is sent as Bearer.'),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        $action = strtolower(trim((string) ($arguments['action'] ?? '')));

        return match ($action) {
            'connect' => $this->connect($arguments),
            'disconnect' => $this->disconnect($arguments),
            'list' => $this->listServers(),
            default => ['success' => false, 'error' => 'Action must be connect, disconnect, or list.'],
        };
    }

    /**
     * @param  array<string, mixed>  $arguments
     * @return array<string, mixed>
     */
    protected function connect(array $arguments): array
    {
        $url = rtrim(trim((string) ($arguments['url'] ?? '')), '/');
        if (! $this->isHttpUrl($url)) {
            return ['success' => false, 'error' => 'Connect requires an http or https MCP URL.'];
        }

        $catalog = app(McpCatalog::class);
        $id = trim((string) ($arguments['id'] ?? ''));
        if ($id === '') {
            $id = $catalog->suggestServerId($url);
        }

        if ($id === HostMcpToolGateway::HOST_SERVER_ID) {
            return ['success' => false, 'error' => 'The lms server is reserved and cannot be replaced by this tool.'];
        }

        $entry = ['url' => $url];
        $authorization = $this->authorizationHeader((string) ($arguments['authorization'] ?? ''));
        if ($authorization !== '') {
            $entry['headers'] = ['Authorization' => $authorization];
        }

        app(VedaSettingsRepository::class)->upsertMcpServer($id, $entry);

        $gateway = app(HostMcpToolGateway::class);
        $gateway->forget();
        $tools = $gateway->tools(RequestContext::current());
        $names = $this->toolNamesForServer($tools, $id);

        ToolDeferralPolicy::$dynamicallyActivatedTools = array_values(array_unique(array_merge(
            ToolDeferralPolicy::$dynamicallyActivatedTools,
            $names,
        )));

        return [
            'success' => true,
            'data' => [
                'id' => $id,
                'url' => $url,
                'tools' => $names,
                'tools_ready' => $names !== [],
            ],
        ];
    }

    /**
     * @param  array<string, mixed>  $arguments
     * @return array<string, mixed>
     */
    protected function disconnect(array $arguments): array
    {
        $id = trim((string) ($arguments['id'] ?? ''));
        $url = rtrim(trim((string) ($arguments['url'] ?? '')), '/');
        if ($id === '' && $this->isHttpUrl($url)) {
            $id = app(McpCatalog::class)->suggestServerId($url);
        }

        if ($id === '' || $id === HostMcpToolGateway::HOST_SERVER_ID) {
            return ['success' => false, 'error' => 'Disconnect requires a catalog id other than lms.'];
        }

        $settings = app(VedaSettingsRepository::class);
        $mcp = app(McpCatalog::class)->normalize($settings->document()['mcp'] ?? []);
        if (! isset($mcp['mcpServers'][$id])) {
            return ['success' => false, 'error' => 'MCP server ['.$id.'] is not in the catalog.'];
        }

        $entry = is_array($mcp['mcpServers'][$id]) ? $mcp['mcpServers'][$id] : [];
        $entry['disabled'] = true;
        $settings->upsertMcpServer($id, $entry);
        app(HostMcpToolGateway::class)->forget();

        return [
            'success' => true,
            'data' => [
                'id' => $id,
                'disabled' => true,
            ],
        ];
    }

    /**
     * @return array<string, mixed>
     */
    protected function listServers(): array
    {
        $masked = app(McpCatalog::class)->mask(
            app(McpCatalog::class)->normalize(app(VedaSettingsRepository::class)->document()['mcp'] ?? [])
        );

        $servers = [];
        foreach ($masked['mcpServers'] as $id => $server) {
            if (! is_array($server)) {
                continue;
            }

            $servers[] = [
                'id' => (string) $id,
                'url' => (string) ($server['url'] ?? ''),
                'command' => (string) ($server['command'] ?? ''),
                'disabled' => (bool) ($server['disabled'] ?? false),
            ];
        }

        return [
            'success' => true,
            'data' => ['servers' => $servers],
        ];
    }

    /**
     * @param  array<int, VedaTool>  $tools
     * @return array<int, string>
     */
    protected function toolNamesForServer(array $tools, string $id): array
    {
        $prefix = $id.'__';
        $names = [];

        foreach ($tools as $tool) {
            $name = $tool instanceof HostMcpVedaTool ? $tool->name() : ToolNameResolver::resolve($tool);
            if (str_starts_with($name, $prefix)) {
                $names[] = $name;
            }
        }

        return $names;
    }

    protected function authorizationHeader(string $value): string
    {
        $value = trim($value);
        if ($value === '') {
            return '';
        }

        if (! str_contains($value, ' ')) {
            return 'Bearer '.$value;
        }

        return $value;
    }

    protected function isHttpUrl(string $url): bool
    {
        return str_starts_with($url, 'http://') || str_starts_with($url, 'https://');
    }
}
