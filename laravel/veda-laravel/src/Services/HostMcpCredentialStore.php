<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Cache;

class HostMcpCredentialStore
{
    /**
     * @return array{url: string, token: string}|null
     */
    public function get(string $visitorId): ?array
    {
        $visitorId = trim($visitorId);
        if ($visitorId === '') {
            return null;
        }

        $value = Cache::get($this->key($visitorId));
        if (! is_array($value)) {
            return null;
        }

        $url = trim((string) ($value['url'] ?? ''));
        $token = trim((string) ($value['token'] ?? ''));
        if ($url === '' || $token === '') {
            return null;
        }

        return [
            'url' => $url,
            'token' => $token,
        ];
    }

    public function put(string $visitorId, string $url, string $token, int $ttlSeconds): void
    {
        $visitorId = trim($visitorId);
        $url = trim($url);
        $token = trim($token);
        if ($visitorId === '' || $url === '' || $token === '') {
            return;
        }

        Cache::put($this->key($visitorId), [
            'url' => $url,
            'token' => $token,
        ], max(60, $ttlSeconds));
    }

    protected function key(string $visitorId): string
    {
        return 'veda.host_mcp.'.hash('sha256', $visitorId);
    }
}
