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
            return redirect()->route('veda.admin');
        }

        if ($request->session()->get('veda.admin') !== true) {
            return redirect()->route('veda.admin');
        }

        return $next($request);
    }
}
