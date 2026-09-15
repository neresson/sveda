<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Support\Facades\Queue;
use Laravel\Ai\Ai;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Services\EmbedTokenService;
use Veda\Laravel\Services\HostMcpCredentialStore;
use Veda\Laravel\Services\VedaSettingsRepository;
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
        $this->assertSame('default', $response->json('appearance.preset'));
        $this->assertSame('0px', $response->json('appearance.radius'));
        $this->assertArrayHasKey('brand', $response->json('appearance.tokens'));
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

    public function test_host_api_key_is_not_required_when_unconfigured(): void
    {
        config()->set('veda.embed.host_api_key', '');

        $this->postJson('/veda/embed/token', ['visitor_id' => 'visitor-open'])
            ->assertOk()
            ->assertJsonPath('visitor_id', 'visitor-open');
    }

    public function test_host_api_key_is_required_when_configured(): void
    {
        config()->set('veda.embed.host_api_key', 'sidecar-host-secret');

        $this->postJson('/veda/embed/token', ['visitor_id' => 'visitor-abc'])
            ->assertStatus(401);
    }

    public function test_issues_embed_token_with_bearer_host_api_key(): void
    {
        config()->set('veda.embed.host_api_key', 'sidecar-host-secret');

        $response = $this->postJson('/veda/embed/token', ['visitor_id' => 'visitor-host'], [
            'Authorization' => 'Bearer sidecar-host-secret',
        ]);

        $response->assertOk();
        $this->assertSame('visitor-host', $response->json('visitor_id'));
        $this->assertNotNull(app(EmbedTokenService::class)->validate((string) $response->json('token')));
    }

    public function test_issues_embed_token_with_host_key_header(): void
    {
        config()->set('veda.embed.host_api_key', 'sidecar-host-secret');

        $this->postJson('/veda/embed/token', ['visitor_id' => 'visitor-header'], [
            'X-Veda-Host-Key' => 'sidecar-host-secret',
        ])->assertOk()->assertJsonPath('visitor_id', 'visitor-header');
    }

    public function test_stores_host_mcp_credentials_when_host_key_is_configured(): void
    {
        config()->set('veda.embed.host_api_key', 'sidecar-host-secret');

        $this->postJson('/veda/embed/token', [
            'visitor_id' => 'visitor-mcp',
            'host_mcp_url' => 'http://127.0.0.1:8001/mcp/veda',
            'host_mcp_token' => 'mcp-secret-token',
        ], [
            'X-Veda-Host-Key' => 'sidecar-host-secret',
        ])->assertOk();

        $stored = app(HostMcpCredentialStore::class)->get('visitor-mcp');
        $this->assertNotNull($stored);
        $this->assertSame('http://127.0.0.1:8001/mcp/veda', $stored['url']);
        $this->assertSame('mcp-secret-token', $stored['token']);

        $mcp = app(VedaSettingsRepository::class)->document()['mcp']['mcpServers'] ?? [];
        $this->assertSame('http://127.0.0.1:8001/mcp/veda', $mcp['lms']['url'] ?? null);
    }

    public function test_embed_token_does_not_replace_an_existing_lms_catalog_entry(): void
    {
        config()->set('veda.embed.host_api_key', 'sidecar-host-secret');
        app(VedaSettingsRepository::class)->update([
            'mcp' => [
                'mcpServers' => [
                    'lms' => [
                        'url' => 'https://lms.example.test/mcp/veda',
                    ],
                ],
            ],
        ]);

        $this->postJson('/veda/embed/token', [
            'visitor_id' => 'visitor-keep-lms',
            'host_mcp_url' => 'http://127.0.0.1:8001/mcp/veda',
            'host_mcp_token' => 'mcp-secret-token',
        ], [
            'X-Veda-Host-Key' => 'sidecar-host-secret',
        ])->assertOk();

        $mcp = app(VedaSettingsRepository::class)->document()['mcp']['mcpServers'] ?? [];
        $this->assertCount(1, $mcp);
        $this->assertSame('https://lms.example.test/mcp/veda', $mcp['lms']['url'] ?? null);
    }

    public function test_ignores_host_mcp_credentials_when_host_api_key_is_unconfigured(): void
    {
        config()->set('veda.embed.host_api_key', '');

        $this->postJson('/veda/embed/token', [
            'visitor_id' => 'visitor-open-mcp',
            'host_mcp_url' => 'http://127.0.0.1:8001/mcp/veda',
            'host_mcp_token' => 'should-not-store',
        ])->assertOk();

        $this->assertNull(app(HostMcpCredentialStore::class)->get('visitor-open-mcp'));
    }

    public function test_rejects_mismatched_host_api_key(): void
    {
        config()->set('veda.embed.host_api_key', 'sidecar-host-secret');

        $this->postJson('/veda/embed/token', ['visitor_id' => 'visitor-abc'], [
            'Authorization' => 'Bearer wrong-secret',
        ])->assertStatus(401);
    }

    public function test_embed_guest_can_stream_without_a_logged_in_user(): void
    {
        Queue::fake();
        config()->set('veda.title_generation.enabled', false);
        config()->set('veda.compaction.enabled', false);

        $token = app(EmbedTokenService::class)->issue('visitor-guest-stream');
        Ai::fakeAgent(VedaAgent::class, ['Hello guest']);

        $response = $this->postJson('/veda/stream', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-guest-1',
            'prompt' => 'Say hello',
        ], [
            'Accept' => 'application/vnd.veda.stream+json',
            'X-Veda-Embed-Token' => $token,
        ]);

        $response->assertOk();
        $this->assertStringContainsString('Hello', $response->streamedContent());
    }
}
