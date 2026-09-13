<?php

namespace Veda\Laravel\Http\Middleware;

use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;

class VedaHandleCors
{
    public function handle(Request $request, Closure $next): Response
    {
        $origins = $this->allowedOrigins();
        if ($origins === []) {
            return $next($request);
        }

        $prefix = trim((string) config('veda.prefix', 'veda'), '/');
        if ($prefix !== '' && ! $request->is($prefix) && ! $request->is($prefix.'/*')) {
            return $next($request);
        }

        if ($request->isMethod('OPTIONS')) {
            $response = response()->noContent();
            $this->applyCorsHeaders($request, $response, $origins);

            return $response;
        }

        $response = $next($request);
        $this->applyCorsHeaders($request, $response, $origins);

        return $response;
    }

    /**
     * @return array<int, string>
     */
    protected function allowedOrigins(): array
    {
        $origins = config('veda.cors.allowed_origins', []);
        if (! is_array($origins)) {
            $origins = array_map('trim', explode(',', (string) $origins));
        }

        return array_values(array_filter(array_map('strval', $origins)));
    }

    /**
     * @param  array<int, string>  $origins
     */
    protected function applyCorsHeaders(Request $request, Response $response, array $origins): void
    {
        $requestOrigin = trim((string) $request->headers->get('Origin', ''));
        $allowOrigin = $this->allowOrigin($requestOrigin, $origins);

        if ($allowOrigin === null) {
            return;
        }

        $headers = (array) config('veda.cors.allowed_headers', []);
        $methods = (array) config('veda.cors.allowed_methods', ['GET', 'POST', 'PATCH', 'DELETE', 'OPTIONS']);

        $response->headers->set('Access-Control-Allow-Origin', $allowOrigin);
        $response->headers->set('Access-Control-Allow-Methods', implode(', ', $methods));
        $response->headers->set('Access-Control-Allow-Headers', implode(', ', $headers));
        $response->headers->set('Vary', 'Origin');
    }

    /**
     * @param  array<int, string>  $origins
     */
    protected function allowOrigin(string $requestOrigin, array $origins): ?string
    {
        if ($requestOrigin === '') {
            return null;
        }

        if (in_array('*', $origins, true)) {
            return '*';
        }

        return in_array($requestOrigin, $origins, true) ? $requestOrigin : null;
    }
}
