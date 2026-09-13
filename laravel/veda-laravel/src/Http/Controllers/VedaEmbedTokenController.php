<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Str;
use Veda\Laravel\Services\EmbedTokenService;

class VedaEmbedTokenController
{
    public function __invoke(Request $request, EmbedTokenService $tokens): JsonResponse
    {
        if (! (bool) config('veda.embed.enabled', false)) {
            abort(404);
        }

        $authorize = config('veda.embed.authorize');
        if (is_callable($authorize) && ! $authorize($request)) {
            abort(403);
        }

        $validated = $request->validate([
            'visitor_id' => 'nullable|string|max:64',
        ]);

        $visitorId = trim((string) ($validated['visitor_id'] ?? ''));
        if ($visitorId === '') {
            $visitorId = (string) Str::uuid();
        }

        $ttl = (int) config('veda.embed.token_ttl_seconds', 86400);
        $token = $tokens->issue($visitorId, $ttl);

        return response()->json([
            'token' => $token,
            'visitor_id' => $visitorId,
            'expires_in' => max(60, $ttl),
        ]);
    }
}
