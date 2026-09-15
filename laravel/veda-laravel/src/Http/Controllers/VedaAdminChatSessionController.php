<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Veda\Laravel\Services\EmbedTokenService;

class VedaAdminChatSessionController
{
    public const VISITOR_ID = 'veda-admin';

    public function __invoke(Request $request, EmbedTokenService $tokens): JsonResponse
    {
        $visitorId = trim((string) $request->session()->get('veda.admin_visitor_id', ''));
        if ($visitorId === '') {
            $visitorId = self::VISITOR_ID;
            $request->session()->put('veda.admin_visitor_id', $visitorId);
        }

        $ttl = max(60, (int) config('veda.embed.token_ttl_seconds', 3600));

        return response()->json([
            'origin' => rtrim($request->getSchemeAndHttpHost(), '/'),
            'token' => $tokens->issue($visitorId, $ttl),
            'expires_in' => $ttl,
        ]);
    }
}
