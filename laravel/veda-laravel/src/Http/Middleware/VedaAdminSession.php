<?php

namespace Veda\Laravel\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;
use Veda\Laravel\Services\VedaAdminAccess;

class VedaAdminSession
{
    public function handle(Request $request, Closure $next): Response
    {
        if (! app(VedaAdminAccess::class)->isConfigured()) {
            return $this->deny($request);
        }

        if ($request->session()->get('veda.admin') !== true) {
            return $this->deny($request);
        }

        return $next($request);
    }

    protected function deny(Request $request): Response
    {
        if ($request->expectsJson()) {
            abort(401);
        }

        return redirect()->route('veda.admin');
    }
}
