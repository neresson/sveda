<?php

namespace Veda\Laravel\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;
use Veda\Laravel\Services\VedaAdminAccess;

class VedaAdminAuth
{
    public function handle(Request $request, Closure $next): Response
    {
        $access = app(VedaAdminAccess::class);
        if (! $access->isConfigured()) {
            abort(404);
        }

        $provided = trim((string) $request->header('X-Veda-Admin-Key', ''));
        if ($provided === '') {
            $bearer = $request->bearerToken();
            $provided = is_string($bearer) ? trim($bearer) : '';
        }

        if (! $access->matches($provided)) {
            abort(401);
        }

        return $next($request);
    }
}
