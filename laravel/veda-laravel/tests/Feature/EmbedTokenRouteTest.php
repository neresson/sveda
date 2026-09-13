<?php

namespace Veda\Laravel\Tests\Feature;

use Veda\Laravel\Services\EmbedTokenService;
use Veda\Laravel\Tests\TestCase;

class EmbedTokenRouteTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.embed.enabled', true);
    }

    public function test_issues_embed_token(): void
    {
        $response = $this->postJson('/veda/embed/token', ['visitor_id' => 'visitor-abc']);

        $response->assertOk();

        $token = (string) $response->json('token');
        $payload = app(EmbedTokenService::class)->validate($token);

        $this->assertNotNull($payload);
        $this->assertSame('visitor-abc', $payload['visitor_id']);
    }

    public function test_embed_token_grants_access_to_history_routes(): void
    {
        $token = app(EmbedTokenService::class)->issue('visitor-xyz');

        $response = $this->getJson('/veda/chat-histories', [
            'X-Veda-Embed-Token' => $token,
        ]);

        $response->assertOk();
        $response->assertJson(['histories' => []]);
    }

    public function test_invalid_embed_token_is_rejected(): void
    {
        $this->getJson('/veda/chat-histories', [
            'X-Veda-Embed-Token' => 'veda_embed_invalid.token',
        ])->assertStatus(401);
    }
}
