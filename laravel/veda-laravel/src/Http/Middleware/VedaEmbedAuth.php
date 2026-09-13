<?php

namespace Veda\Laravel\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;
use Symfony\Component\HttpFoundation\Response;
use Veda\Laravel\Services\EmbedTokenService;

class VedaEmbedAuth
{
    public const ATTRIBUTE_EMBED_GUEST = 'veda.embed_guest';

    public const ATTRIBUTE_EMBED_MODE = 'veda.embed_mode';

    public const ATTRIBUTE_VISITOR_ID = 'veda.visitor_id';

    public function __construct(
        protected EmbedTokenService $tokens,
    ) {}

    public function handle(Request $request, Closure $next): Response
    {
        if ($request->user()) {
            return $next($request);
        }

        $token = $this->extractToken($request);

        if ($token === null) {
            abort(401);
        }

        $payload = $this->tokens->validate($token);

        if ($payload === null) {
            abort(401);
        }

        $guestResolver = config('veda.embed.user_resolver');
        $guest = is_callable($guestResolver) ? $guestResolver($request, $payload['visitor_id']) : null;

        if ($guest !== null) {
            $request->setUserResolver(fn () => $guest);
            Auth::guard()->setUser($guest);
        }

        $request->attributes->set(self::ATTRIBUTE_EMBED_GUEST, true);
        $request->attributes->set(self::ATTRIBUTE_EMBED_MODE, true);
        $request->attributes->set(self::ATTRIBUTE_VISITOR_ID, $payload['visitor_id']);

        return $next($request);
    }

    protected function extractToken(Request $request): ?string
    {
        $header = trim((string) $request->header('X-Veda-Embed-Token', ''));
        if ($header !== '') {
            return $header;
        }

        $bearer = $request->bearerToken();
        if (is_string($bearer) && str_starts_with($bearer, EmbedTokenService::TOKEN_PREFIX)) {
            return $bearer;
        }

        return null;
    }
}
