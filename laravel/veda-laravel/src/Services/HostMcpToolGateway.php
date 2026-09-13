<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Log;
use Laravel\Mcp\Client;
use Laravel\Mcp\WebClient;
use Throwable;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;
use Veda\Laravel\Tools\HostMcpVedaTool;
use Veda\Laravel\Tools\VedaTool;

class HostMcpToolGateway
{
    public const PAGE_CONTEXT_HEADER = 'X-Veda-Page-Context';

    public const CHAT_ID_HEADER = 'X-Veda-Chat-Id';

    protected const MAX_PAGE_CONTEXT_BYTES = 24000;

    /**
     * @var array<int, VedaTool>|null
     */
    protected ?array $resolvedTools = null;

    protected ?WebClient $client = null;

    public function __construct(protected HostMcpCredentialStore $credentials) {}

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

        $visitorId = request()->attributes->get(VedaEmbedAuth::ATTRIBUTE_VISITOR_ID);
        if (! is_string($visitorId) || trim($visitorId) === '') {
            return $this->resolvedTools = [];
        }

        $creds = $this->credentials->get($visitorId);
        if ($creds === null) {
            return $this->resolvedTools = [];
        }

        try {
            $client = Client::web($creds['url'])
                ->withToken($creds['token'])
                ->withTimeout((float) config('veda.host.mcp.timeout', 30));

            $headers = $this->requestHeaders($context);
            if ($headers !== []) {
                $client->withHeaders($headers);
            }

            $this->client = $client;

            $this->resolvedTools = $client->tools()
                ->map(fn ($tool): HostMcpVedaTool => new HostMcpVedaTool($tool))
                ->values()
                ->all();

            return $this->resolvedTools;
        } catch (Throwable $e) {
            Log::warning('veda.host_mcp.tools_failed', ['message' => $e->getMessage()]);

            return $this->resolvedTools = [];
        }
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
