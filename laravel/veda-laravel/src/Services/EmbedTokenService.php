<?php

namespace Veda\Laravel\Services;

class EmbedTokenService
{
    public const TOKEN_PREFIX = 'veda_embed_';

    public function issue(string $visitorId, ?int $ttlSeconds = null): string
    {
        $ttl = $ttlSeconds ?? (int) config('veda.embed.token_ttl_seconds', 86400);
        $expiresAt = time() + max(60, $ttl);

        $payload = [
            'visitor_id' => $visitorId,
            'expires_at' => $expiresAt,
        ];

        $encoded = rtrim(strtr(base64_encode((string) json_encode($payload)), '+/', '-_'), '=');

        return self::TOKEN_PREFIX.$encoded.'.'.$this->signature($encoded);
    }

    /**
     * @return array{visitor_id: string, expires_at: int}|null
     */
    public function validate(string $token): ?array
    {
        if (! str_starts_with($token, self::TOKEN_PREFIX)) {
            return null;
        }

        $body = substr($token, strlen(self::TOKEN_PREFIX));
        $separator = strrpos($body, '.');
        if ($separator === false) {
            return null;
        }

        $encoded = substr($body, 0, $separator);
        $signature = substr($body, $separator + 1);

        if (! hash_equals($this->signature($encoded), $signature)) {
            return null;
        }

        $decoded = base64_decode(strtr($encoded, '-_', '+/'), true);
        if ($decoded === false) {
            return null;
        }

        $payload = json_decode($decoded, true);
        if (! is_array($payload)) {
            return null;
        }

        $visitorId = (string) ($payload['visitor_id'] ?? '');
        $expiresAt = (int) ($payload['expires_at'] ?? 0);

        if ($visitorId === '' || mb_strlen($visitorId) > 64) {
            return null;
        }

        if ($expiresAt < time()) {
            return null;
        }

        return [
            'visitor_id' => $visitorId,
            'expires_at' => $expiresAt,
        ];
    }

    protected function signature(string $encodedPayload): string
    {
        $key = (string) config('app.key');
        if (str_starts_with($key, 'base64:')) {
            $key = (string) base64_decode(substr($key, 7), true);
        }

        return hash_hmac('sha256', $encodedPayload, $key);
    }
}
