<?php

namespace Veda\Laravel\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

class VedaAuthorize
{
    public function handle(Request $request, Closure $next): Response
    {
        $authorize = config('veda.authorize');

        if (is_callable($authorize)) {
            if (! $authorize($request)) {
                abort(403);
            }

            return $next($request);
        }

        if ($request->user()) {
            return $next($request);
        }

        if ($request->attributes->get(VedaEmbedAuth::ATTRIBUTE_EMBED_GUEST) === true) {
            return $next($request);
        }

        abort(401);
    }
}
