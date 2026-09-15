<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Log;
use Laravel\Mcp\Client;
use Laravel\Mcp\WebClient;
use Throwable;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;
use Veda\Laravel\Mcp\EnvStdioTransport;
use Veda\Laravel\Tools\HostMcpVedaTool;
use Veda\Laravel\Tools\VedaTool;

class HostMcpToolGateway
{
    public const PAGE_CONTEXT_HEADER = 'X-Veda-Page-Context';

    public const CHAT_ID_HEADER = 'X-Veda-Chat-Id';

    public const HOST_SERVER_ID = 'lms';

    protected const MAX_PAGE_CONTEXT_BYTES = 24000;

    /**
     * @var array<int, VedaTool>|null
     */
    protected ?array $resolvedTools = null;

    protected ?WebClient $client = null;

    public function __construct(
        protected HostMcpCredentialStore $credentials,
        protected VedaSettingsRepository $settings,
        protected McpCatalog $catalog,
    ) {}

    /**
     * @return array<int, VedaTool>
     */
    public function tools(?RequestContext $context = null): array
    {
        if ($this->resolvedTools !== null) {
            return $this->resolvedTools;
        }

        if (! class_exists(Client::class)) {
            return $this->resolvedTools = [];
        }

        $tools = [];
        foreach ($this->configuredServers() as $server) {
            $tools = [...$tools, ...$this->toolsForServer($server, $context)];
        }

        return $this->resolvedTools = $tools;
    }

    public function forget(): void
    {
        $this->resolvedTools = null;
        $this->client = null;
    }

    /**
     * @param  array{id: string, url: string, command: string, args: array<int, string>, headers: array<string, string>, env: array<string, string>, envFile: string, cwd: string}  $server
     * @return array<int, VedaTool>
     */
    protected function toolsForServer(array $server, ?RequestContext $context): array
    {
        if ($server['command'] !== '') {
            return $this->stdioTools($server);
        }

        if (! $this->isHttpUrl($server['url'])) {
            return [];
        }

        $headers = $server['headers'];
        $prefix = $server['id'];
        $token = '';

        if ($server['id'] === self::HOST_SERVER_ID) {
            $creds = $this->sessionCredentials();
            if ($creds === null) {
                return [];
            }

            $token = $creds['token'];
            unset($headers['Authorization'], $headers['authorization']);
            $headers = [...$headers, ...$this->requestHeaders($context)];
            $prefix = null;
        }

        try {
            $client = Client::web($server['url'])->withTimeout($this->timeout());
            if ($token !== '') {
                $client->withToken($token);
            }
            if ($headers !== []) {
                $client->withHeaders($headers);
            }

            if ($server['id'] === self::HOST_SERVER_ID) {
                $this->client = $client;
            }

            return $this->wrapTools($client, $prefix);
        } catch (Throwable $e) {
            Log::warning('veda.mcp_server.tools_failed', [
                'server' => $server['id'],
                'message' => $e->getMessage(),
            ]);

            return [];
        }
    }

    /**
     * @param  array{id: string, command: string, args: array<int, string>, env: array<string, string>, envFile: string, cwd: string}  $server
     * @return array<int, VedaTool>
     */
    protected function stdioTools(array $server): array
    {
        try {
            $client = (new Client(new EnvStdioTransport(
                $server['command'],
                $server['args'],
                $this->processEnv($server),
                $server['cwd'],
            )))->withTimeout($this->timeout());

            $prefix = $server['id'] === self::HOST_SERVER_ID ? null : $server['id'];

            return $this->wrapTools($client, $prefix);
        } catch (Throwable $e) {
            Log::warning('veda.mcp_server.tools_failed', [
                'server' => $server['id'],
                'message' => $e->getMessage(),
            ]);

            return [];
        }
    }

    /**
     * @return array<int, VedaTool>
     */
    protected function wrapTools(Client $client, ?string $prefix): array
    {
        return $client->tools()
            ->map(fn ($tool): HostMcpVedaTool => new HostMcpVedaTool($tool, $prefix))
            ->values()
            ->all();
    }

    /**
     * @param  array{env: array<string, string>, envFile: string}  $server
     * @return array<string, string>
     */
    protected function processEnv(array $server): array
    {
        return [...$this->catalog->loadEnvFile($server['envFile']), ...$server['env']];
    }

    /**
     * @return array<int, array{id: string, url: string, command: string, args: array<int, string>, headers: array<string, string>, env: array<string, string>, envFile: string, cwd: string}>
     */
    protected function configuredServers(): array
    {
        $mcp = $this->catalog->normalize($this->settings->document()['mcp'] ?? []);
        $servers = [];

        foreach ($mcp['mcpServers'] as $id => $server) {
            if (! is_array($server) || filter_var($server['disabled'] ?? false, FILTER_VALIDATE_BOOLEAN)) {
                continue;
            }

            $id = trim((string) $id);
            $url = $this->catalog->interpolate(rtrim(trim((string) ($server['url'] ?? '')), '/'));
            $command = $this->catalog->interpolate(trim((string) ($server['command'] ?? '')));
            if ($id === '' || ($url === '' && $command === '')) {
                continue;
            }

            $args = [];
            foreach ((array) ($server['args'] ?? []) as $arg) {
                $args[] = $this->catalog->interpolate(trim((string) $arg));
            }

            $servers[] = [
                'id' => $id,
                'url' => $url,
                'command' => $command,
                'args' => array_values(array_filter($args, fn (string $arg): bool => $arg !== '')),
                'headers' => $this->catalog->interpolateMap(is_array($server['headers'] ?? null) ? $server['headers'] : []),
                'env' => $this->catalog->interpolateMap(is_array($server['env'] ?? null) ? $server['env'] : []),
                'envFile' => $this->catalog->interpolate(trim((string) ($server['envFile'] ?? ''))),
                'cwd' => $this->catalog->interpolate(trim((string) ($server['cwd'] ?? ''))),
            ];
        }

        return $servers;
    }

    /**
     * @return array{url: string, token: string}|null
     */
    protected function sessionCredentials(): ?array
    {
        $visitorId = request()->attributes->get(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID);
        if (! is_string($visitorId) || trim($visitorId) === '') {
            return null;
        }

        return $this->credentials->get($visitorId);
    }

    protected function isHttpUrl(string $url): bool
    {
        return str_starts_with($url, 'http://') || str_starts_with($url, 'https://');
    }

    protected function timeout(): float
    {
        return (float) config('veda.host.mcp.timeout', 30);
    }

    /**
     * @return array<string, string>
     */
    protected function requestHeaders(?RequestContext $context): array
    {
        $headers = [];
        $pageContext = $context?->fullPageContext() ?? [];
        if ($pageContext === []) {
            $raw = request()->input('context', []);
            $pageContext = is_array($raw) ? (new PageContextSanitizer)->sanitize($raw) : [];
        }

        if ($pageContext !== []) {
            $encoded = json_encode($pageContext, JSON_UNESCAPED_UNICODE);
            if (is_string($encoded) && strlen($encoded) <= self::MAX_PAGE_CONTEXT_BYTES) {
                $headers[self::PAGE_CONTEXT_HEADER] = $encoded;
            }
        }

        $chatId = $context?->chatId ?? request()->input('chatId');
        if (is_string($chatId) && trim($chatId) !== '') {
            $headers[self::CHAT_ID_HEADER] = trim($chatId);
        }

        return $headers;
    }
}
