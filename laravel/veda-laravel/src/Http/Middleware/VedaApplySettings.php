<?php

namespace Veda\Laravel\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;
use Veda\Laravel\Services\VedaSettingsRepository;

class VedaApplySettings
{
    public function handle(Request $request, Closure $next): Response
    {
        app(VedaSettingsRepository::class)->applyToConfig();

        return $next($request);
    }
}
