<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Str;
use Veda\Laravel\Services\EmbedTokenService;
use Veda\Laravel\Services\HostMcpCredentialStore;

class VedaEmbedTokenController
{
    public function __invoke(Request $request, EmbedTokenService $tokens): JsonResponse
    {
        if (! (bool) config('veda.embed.enabled', false)) {
            abort(404);
        }

        $this->assertHostApiKey($request);

        $authorize = config('veda.embed.authorize');
        if (is_callable($authorize) && ! $authorize($request)) {
            abort(403);
        }

        $validated = $request->validate([
            'visitor_id' => 'nullable|string|max:64',
            'host_mcp_url' => 'nullable|url|max:2048|required_with:host_mcp_token',
            'host_mcp_token' => 'nullable|string|max:2048|required_with:host_mcp_url',
        ]);

        $visitorId = trim((string) ($validated['visitor_id'] ?? ''));
        if ($visitorId === '') {
            $visitorId = (string) Str::uuid();
        }

        $ttl = (int) config('veda.embed.token_ttl_seconds', 86400);
        $token = $tokens->issue($visitorId, $ttl);

        $hostMcpUrl = trim((string) ($validated['host_mcp_url'] ?? ''));
        $hostMcpToken = trim((string) ($validated['host_mcp_token'] ?? ''));
        if ($hostMcpUrl !== '' && $hostMcpToken !== '' && trim((string) config('veda.embed.host_api_key', '')) !== '') {
            app(HostMcpCredentialStore::class)->put(
                $visitorId,
                $hostMcpUrl,
                $hostMcpToken,
                $ttl,
            );
        }

        return response()->json([
            'token' => $token,
            'visitor_id' => $visitorId,
            'expires_in' => max(60, $ttl),
        ]);
    }

    protected function assertHostApiKey(Request $request): void
    {
        $configured = trim((string) config('veda.embed.host_api_key', ''));
        if ($configured === '') {
            return;
        }

        $provided = trim((string) $request->header('X-Veda-Host-Key', ''));
        if ($provided === '') {
            $bearer = $request->bearerToken();
            $provided = is_string($bearer) ? trim($bearer) : '';
        }

        if ($provided === '' || ! hash_equals($configured, $provided)) {
            abort(401);
        }
    }
}
