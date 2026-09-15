<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Veda\Laravel\Http\Controllers\VedaAdminChatSessionController;
use Veda\Laravel\Services\EmbedTokenService;
use Veda\Laravel\Tests\TestCase;

class AdminChatSessionTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.admin.api_key', 'veda-admin-secret');
        $app['config']->set('veda.embed.enabled', false);
        $app['config']->set('veda.embed.token_ttl_seconds', 120);
    }

    protected function setUp(): void
    {
        parent::setUp();

        $this->withoutMiddleware(ValidateCsrfToken::class);
    }

    public function test_guest_cannot_issue_admin_chat_session(): void
    {
        $this->postJson('/veda/admin/chat-session')->assertUnauthorized();
    }

    public function test_admin_session_issues_embed_token_for_self_hosted_chat(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $response = $this->postJson('/veda/admin/chat-session');

        $response
            ->assertOk()
            ->assertJsonPath('expires_in', 120)
            ->assertJsonStructure(['origin', 'token', 'expires_in']);

        $token = (string) $response->json('token');
        $payload = app(EmbedTokenService::class)->validate($token);

        $this->assertNotNull($payload);
        $this->assertSame(VedaAdminChatSessionController::VISITOR_ID, $payload['visitor_id']);
    }

    public function test_admin_embed_token_grants_access_to_chat_histories(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $token = (string) $this->postJson('/veda/admin/chat-session')->json('token');

        $this->getJson('/veda/chat-histories', [
            'X-Veda-Embed-Token' => $token,
        ])
            ->assertOk()
            ->assertJson(['histories' => []]);
    }

    public function test_dashboard_payload_includes_self_hosted_chat_session(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->get('/veda/admin')
            ->assertOk()
            ->assertViewHas('chat', function (array $chat): bool {
                return str_contains((string) ($chat['sessionUrl'] ?? ''), '/veda/admin/chat-session')
                    && ($chat['prefix'] ?? '') === 'veda'
                    && ($chat['protocol'] ?? '') === 'veda'
                    && isset($chat['models'][0]['id'])
                    && ($chat['models'][0]['supportsThinking'] ?? false) === true;
            });
    }
}
