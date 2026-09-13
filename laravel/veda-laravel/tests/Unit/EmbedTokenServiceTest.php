<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Services\EmbedTokenService;
use Veda\Laravel\Tests\TestCase;

class EmbedTokenServiceTest extends TestCase
{
    public function test_issue_and_validate_roundtrip(): void
    {
        $service = app(EmbedTokenService::class);

        $token = $service->issue('visitor-123');
        $payload = $service->validate($token);

        $this->assertNotNull($payload);
        $this->assertSame('visitor-123', $payload['visitor_id']);
    }

    public function test_rejects_tampered_token(): void
    {
        $service = app(EmbedTokenService::class);

        $token = $service->issue('visitor-123');
        $tampered = $token.'x';

        $this->assertNull($service->validate($tampered));
    }

    public function test_rejects_garbage_token(): void
    {
        $service = app(EmbedTokenService::class);

        $this->assertNull($service->validate('not-a-token'));
    }

    public function test_rejects_expired_token(): void
    {
        $service = app(EmbedTokenService::class);

        $payload = ['visitor_id' => 'visitor-123', 'expires_at' => time() - 5];
        $encoded = rtrim(strtr(base64_encode((string) json_encode($payload)), '+/', '-_'), '=');

        $key = (string) config('app.key');
        if (str_starts_with($key, 'base64:')) {
            $key = (string) base64_decode(substr($key, 7), true);
        }

        $signature = hash_hmac('sha256', $encoded, $key);
        $token = EmbedTokenService::TOKEN_PREFIX.$encoded.'.'.$signature;

        $this->assertNull($service->validate($token));
    }
}
