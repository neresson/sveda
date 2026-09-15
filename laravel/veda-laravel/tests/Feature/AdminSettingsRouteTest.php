<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Veda\Laravel\Services\EmbedTokenService;
use Veda\Laravel\Services\VedaSettingsRepository;
use Veda\Laravel\Tests\TestCase;

class AdminSettingsRouteTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.admin.api_key', 'veda-admin-secret');
        $app['config']->set('veda.embed.enabled', true);
        $app['config']->set('veda.embed.host_api_key', 'sidecar-host-secret');
        $app['config']->set('veda.cors.allowed_origins', ['http://localhost:8001']);
        $app['config']->set('veda.deepseek.key', 'deepseek-live-key');
    }

    public function test_admin_settings_are_not_found_when_admin_key_is_unconfigured(): void
    {
        config()->set('veda.admin.api_key', '');

        $this->getJson('/veda/admin/settings')
            ->assertNotFound();
    }

    public function test_json_api_accepts_admin_key_created_via_setup(): void
    {
        config()->set('veda.admin.api_key', '');
        $this->withoutMiddleware(ValidateCsrfToken::class);

        $this->post('/veda/admin/setup', [
            'key' => 'veda-admin-secret-from-setup',
            'key_confirmation' => 'veda-admin-secret-from-setup',
        ])->assertRedirect('/veda/admin');

        $this->getJson('/veda/admin/settings', [
            'X-Veda-Admin-Key' => 'veda-admin-secret-from-setup',
        ])->assertOk();
    }

    public function test_admin_settings_require_admin_key(): void
    {
        $this->getJson('/veda/admin/settings')
            ->assertStatus(401);
    }

    public function test_embed_host_key_cannot_read_admin_settings(): void
    {
        $this->getJson('/veda/admin/settings', [
            'Authorization' => 'Bearer sidecar-host-secret',
        ])->assertStatus(401);
    }

    public function test_reads_settings_with_admin_key_header(): void
    {
        $response = $this->getJson('/veda/admin/settings', [
            'X-Veda-Admin-Key' => 'veda-admin-secret',
        ]);

        $response->assertOk();
        $response->assertJsonPath('default_model', config('veda.default_model'));
        $response->assertJsonPath('max_steps', (int) config('veda.max_steps'));
        $this->assertSame('••••••••', $response->json('deepseek.key'));
        $this->assertNotSame('deepseek-live-key', $response->json('deepseek.key'));
        $this->assertArrayHasKey('welcome_message', $response->json());
        $this->assertArrayHasKey('system_prompt', $response->json());
        $this->assertArrayHasKey('models', $response->json());
        $this->assertArrayHasKey('compaction', $response->json());
        $this->assertArrayHasKey('cors', $response->json());
        $this->assertArrayHasKey('failover', $response->json());
        $this->assertArrayHasKey('mcp', $response->json());
        $this->assertArrayHasKey('appearance', $response->json());
        $this->assertSame('default', $response->json('appearance.preset'));
        $this->assertSame('0px', $response->json('appearance.radius'));
        $this->assertContains($response->json('models.0.key'), ['', '••••••••']);
    }

    public function test_reads_settings_with_bearer_admin_key(): void
    {
        $this->getJson('/veda/admin/settings', [
            'Authorization' => 'Bearer veda-admin-secret',
        ])->assertOk();
    }

    public function test_updates_settings_and_masks_secrets_on_read(): void
    {
        $this->putJson('/veda/admin/settings', [
            'default_model' => 'custom-responses',
            'failover' => ['deepseek-v4-flash-anthropic'],
            'deepseek' => [
                'key' => '••••••••',
            ],
            'models' => [
                [
                    'id' => 'custom-responses',
                    'label' => 'Custom Responses',
                    'protocol' => 'responses',
                    'api_model' => 'gpt-test',
                    'url' => 'https://example.test/v1',
                    'key' => 'custom-secret-key',
                    'thinking' => false,
                    'vision' => false,
                ],
            ],
            'max_steps' => 18,
            'compaction' => [
                'enabled' => false,
                'min_messages' => 12,
                'keep_tail_messages' => 6,
            ],
            'cors' => [
                'allowed_origins' => ['https://app.example.test'],
            ],
            'welcome_message' => 'Welcome to Acme copilot',
            'system_prompt' => 'Always answer in Russian.',
        ], [
            'X-Veda-Admin-Key' => 'veda-admin-secret',
        ])->assertOk();

        $this->assertSame('custom-responses', config('veda.default_model'));
        $this->assertSame('custom-responses', config('veda.model'));
        $this->assertSame(18, (int) config('veda.max_steps'));
        $this->assertFalse((bool) config('veda.compaction.enabled'));
        $this->assertSame(12, (int) config('veda.compaction.min_messages'));
        $this->assertSame(['https://app.example.test'], config('veda.cors.allowed_origins'));
        $this->assertSame('Welcome to Acme copilot', config('veda.welcome_message'));
        $this->assertSame('Always answer in Russian.', config('veda.system_prompt'));
        $this->assertSame('deepseek-live-key', config('veda.deepseek.key'));
        $this->assertSame('custom-secret-key', config('veda.models.0.key'));
        $this->assertSame('veda-responses', config('ai.providers.veda-model:custom-responses.driver'));

        $read = $this->getJson('/veda/admin/settings', [
            'X-Veda-Admin-Key' => 'veda-admin-secret',
        ]);

        $read->assertOk();
        $read->assertJsonPath('default_model', 'custom-responses');
        $read->assertJsonPath('welcome_message', 'Welcome to Acme copilot');
        $this->assertSame('••••••••', $read->json('deepseek.key'));
        $this->assertSame('••••••••', $read->json('models.0.key'));
    }

    public function test_public_embed_config_omits_secrets(): void
    {
        $this->putJson('/veda/admin/settings', [
            'default_model' => 'deepseek-v4-flash-responses',
            'models' => [
                [
                    'id' => 'deepseek-v4-flash-responses',
                    'label' => 'DeepSeek Flash',
                    'protocol' => 'responses',
                    'api_model' => 'deepseek-v4-flash',
                    'url' => 'https://api.deepseek.com',
                    'key' => 'must-not-leak',
                    'thinking' => true,
                ],
            ],
            'welcome_message' => 'Hi from Veda',
            'system_prompt' => 'Secret host instructions',
            'deepseek' => [
                'key' => 'must-not-leak-shared',
            ],
            'mcp' => [
                'mcpServers' => [
                    'docs' => [
                        'url' => 'https://docs.example.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer must-not-leak-mcp',
                        ],
                    ],
                ],
            ],
        ], [
            'X-Veda-Admin-Key' => 'veda-admin-secret',
        ])->assertOk();

        $token = app(EmbedTokenService::class)->issue('visitor-config');

        $response = $this->getJson('/veda/embed/config', [
            'X-Veda-Embed-Token' => $token,
        ]);

        $response->assertOk();
        $response->assertJsonPath('welcome_message', 'Hi from Veda');
        $response->assertJsonPath('default_model', 'deepseek-v4-flash-responses');
        $response->assertJsonPath('model', 'deepseek-v4-flash-responses');
        $response->assertJsonPath('models.0.id', 'deepseek-v4-flash-responses');
        $response->assertJsonPath('models.0.protocol', 'responses');
        $response->assertJsonPath('appearance.preset', 'default');
        $response->assertJsonPath('appearance.radius', '0px');
        $this->assertTrue((bool) $response->json('models.0.supportsThinking'));
        $this->assertArrayNotHasKey('deepseek', $response->json());
        $this->assertArrayNotHasKey('system_prompt', $response->json());
        $this->assertArrayNotHasKey('mcp', $response->json());
        $this->assertArrayNotHasKey('mcp_servers', $response->json());
        $this->assertStringNotContainsString('must-not-leak', (string) $response->getContent());
    }

    public function test_public_embed_config_includes_saved_appearance(): void
    {
        $this->putJson('/veda/admin/settings', [
            'appearance' => [
                'preset' => 'lms',
                'theme' => 'dark',
            ],
        ], [
            'X-Veda-Admin-Key' => 'veda-admin-secret',
        ])
            ->assertOk()
            ->assertJsonPath('appearance.preset', 'lms')
            ->assertJsonMissingPath('appearance.theme');

        $token = app(EmbedTokenService::class)->issue('visitor-appearance');

        $this->getJson('/veda/embed/config', [
            'X-Veda-Embed-Token' => $token,
        ])
            ->assertOk()
            ->assertJsonPath('appearance.preset', 'lms')
            ->assertJsonPath('appearance.radius', '0px')
            ->assertJsonPath('appearance.tokens.brand', '275 96% 52%');
    }

    public function test_public_embed_config_includes_saved_launcher(): void
    {
        $this->putJson('/veda/admin/settings', [
            'appearance' => [
                'preset' => 'lms',
                'launcher' => [
                    'label' => 'Ask Veda',
                    'icon' => 'bot',
                ],
            ],
        ], [
            'X-Veda-Admin-Key' => 'veda-admin-secret',
        ])
            ->assertOk()
            ->assertJsonPath('appearance.launcher.label', 'Ask Veda')
            ->assertJsonPath('appearance.launcher.icon', 'bot');

        $token = app(EmbedTokenService::class)->issue('visitor-launcher');

        $this->getJson('/veda/embed/config', [
            'X-Veda-Embed-Token' => $token,
        ])
            ->assertOk()
            ->assertJsonPath('appearance.launcher.label', 'Ask Veda')
            ->assertJsonPath('appearance.launcher.icon', 'bot');
    }

    public function test_public_embed_config_includes_saved_launcher_image(): void
    {
        $png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';

        $this->putJson('/veda/admin/settings', [
            'appearance' => [
                'preset' => 'lms',
                'launcher' => [
                    'label' => 'Ask Veda',
                    'icon' => 'bot',
                    'image' => $png,
                ],
            ],
        ], [
            'X-Veda-Admin-Key' => 'veda-admin-secret',
        ])
            ->assertOk()
            ->assertJsonPath('appearance.launcher.image', $png);

        $token = app(EmbedTokenService::class)->issue('visitor-launcher-image');

        $this->getJson('/veda/embed/config', [
            'X-Veda-Embed-Token' => $token,
        ])
            ->assertOk()
            ->assertJsonPath('appearance.launcher.image', $png);
    }

    public function test_public_embed_config_requires_embed_token(): void
    {
        $this->getJson('/veda/embed/config')->assertStatus(401);
    }

    public function test_apply_does_not_override_config_until_settings_are_saved(): void
    {
        config()->set('veda.default_model', 'keep-me');

        app(VedaSettingsRepository::class)->applyToConfig();

        $this->assertSame('keep-me', config('veda.default_model'));
    }
}
