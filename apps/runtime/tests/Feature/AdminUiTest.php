<?php

namespace Tests\Feature;

use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;
use Veda\Laravel\Models\VedaGeneration;

class AdminUiTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        config()->set('veda.admin.api_key', 'veda-admin-secret');
        $this->withoutMiddleware(ValidateCsrfToken::class);
    }

    public function test_setup_page_renders_when_admin_key_is_missing(): void
    {
        config()->set('veda.admin.api_key', '');

        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"setup"', false)
            ->assertSee('veda\\/admin\\/setup', false);
    }

    public function test_login_page_renders_editorial_form(): void
    {
        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"login"', false)
            ->assertSee('veda\\/admin\\/login', false)
            ->assertDontSee('veda\\/admin\\/chat-session', false);
    }

    public function test_login_rejects_wrong_key(): void
    {
        $this->from('/veda/admin')->post('/veda/admin/login', [
            'key' => 'wrong',
        ])->assertRedirect('/veda/admin');
    }

    public function test_dashboard_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"dashboard"', false)
            ->assertSee('veda\\/admin\\/usage', false)
            ->assertSee('veda\\/admin\\/runtime', false)
            ->assertSee('veda\\/admin\\/models', false)
            ->assertSee('veda\\/admin\\/mcp', false)
            ->assertSee('veda\\/admin\\/prompts', false)
            ->assertSee('veda\\/admin\\/appearance', false)
            ->assertSee('veda\\/admin\\/sources', false)
            ->assertSee('/veda/admin/settings', false)
            ->assertSee('veda\\/admin\\/chat-session', false)
            ->assertSee('"prefix":"veda"', false)
            ->assertSee('"protocol":"veda"', false)
            ->assertSee('"period_days":14', false)
            ->assertSee('"requests":0', false);
    }

    public function test_runtime_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin/runtime')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"runtime"', false)
            ->assertSee('veda\\/admin\\/models', false);
    }

    public function test_dashboard_includes_generation_stats(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $generation = VedaGeneration::query()->create([
            'generation_type' => 'chat',
            'prompt' => 'hello',
            'status' => VedaGeneration::STATUS_COMPLETED,
            'prompt_tokens' => 11,
            'completion_tokens' => 22,
            'tokens_used' => 33,
        ]);

        $response = $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('"page":"dashboard"', false)
            ->assertSee('"prompt_tokens":11', false)
            ->assertSee('"completion_tokens":22', false)
            ->assertSee('"tokens_used":33', false);

        $this->assertStringContainsString('"requests":1', $response->getContent());
        $this->assertModelExists($generation);
    }

    public function test_usage_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $generation = VedaGeneration::query()->create([
            'generation_type' => 'chat',
            'model' => 'deepseek-v4-flash',
            'prompt' => 'do-not-leak-this-prompt',
            'status' => VedaGeneration::STATUS_COMPLETED,
            'tokens_used' => 42,
        ]);

        $response = $this->get('/veda/admin/usage')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"usage"', false)
            ->assertSee('"by_model"', false)
            ->assertSee('"deepseek-v4-flash"', false)
            ->assertSee('"tokens_used":42', false)
            ->assertDontSee('do-not-leak-this-prompt');

        $this->assertModelExists($generation);
    }

    public function test_models_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin/models')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"models"', false);
    }

    public function test_mcp_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin/mcp')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"mcp"', false);
    }

    public function test_prompts_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin/prompts')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"prompts"', false);
    }

    public function test_appearance_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin/appearance')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"appearance"', false)
            ->assertSee('"appearancePresets"', false)
            ->assertSee('"preset":"default"', false);
    }

    public function test_sources_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin/sources')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"sources"', false)
            ->assertSee('veda\\/admin\\/code-index\\/sources', false);
    }

    public function test_guest_cannot_issue_admin_chat_session(): void
    {
        $this->postJson('/veda/admin/chat-session')->assertUnauthorized();
    }

    public function test_admin_can_issue_chat_session_and_read_histories(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $token = (string) $this->postJson('/veda/admin/chat-session')
            ->assertOk()
            ->assertJsonStructure(['origin', 'token', 'expires_in'])
            ->json('token');

        $this->getJson('/veda/chat-histories', [
            'X-Veda-Embed-Token' => $token,
        ])
            ->assertOk()
            ->assertJson(['histories' => []]);
    }

    public function test_unknown_admin_section_is_not_found(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->get('/veda/admin/unknown')->assertNotFound();
    }

    public function test_session_can_add_a_model(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'runtime-flash',
                    'label' => 'Runtime Flash',
                    'protocol' => 'responses',
                    'api_model' => 'flash',
                    'url' => 'https://api.example.test',
                    'key' => 'runtime-secret',
                    'thinking' => true,
                    'vision' => false,
                ],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('models.0.id', 'runtime-flash')
            ->assertJsonPath('models.0.label', 'Runtime Flash')
            ->assertJsonPath('models.0.key', '••••••••');
    }

    public function test_session_can_update_an_existing_model_without_replacing_the_key(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'runtime-flash',
                    'label' => 'Runtime Flash',
                    'protocol' => 'responses',
                    'api_model' => 'flash',
                    'url' => 'https://api.example.test',
                    'key' => 'runtime-secret',
                    'thinking' => true,
                    'vision' => false,
                ],
            ],
        ])->assertOk();

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'runtime-flash',
                    'label' => 'Runtime Flash Updated',
                    'protocol' => 'anthropic',
                    'api_model' => 'flash-2',
                    'url' => 'https://api.example.test/v2',
                    'key' => '',
                    'thinking' => false,
                    'vision' => true,
                    'aliases' => ['flash'],
                ],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('models.0.id', 'runtime-flash')
            ->assertJsonPath('models.0.label', 'Runtime Flash Updated')
            ->assertJsonPath('models.0.protocol', 'anthropic')
            ->assertJsonPath('models.0.api_model', 'flash-2')
            ->assertJsonPath('models.0.vision', true)
            ->assertJsonPath('models.0.key', '••••••••');
    }

    public function test_session_can_update_runtime_settings_without_replacing_models(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'runtime-flash',
                    'label' => 'Runtime Flash',
                    'protocol' => 'responses',
                    'api_model' => 'flash',
                    'url' => 'https://api.example.test',
                    'key' => 'runtime-secret',
                    'thinking' => true,
                    'vision' => false,
                ],
            ],
        ])->assertOk();

        $this->postJson('/veda/admin/settings', [
            'default_model' => 'runtime-flash',
            'failover' => ['deepseek-v4-pro'],
            'max_steps' => 12,
            'compaction' => [
                'enabled' => false,
                'min_messages' => 10,
                'keep_tail_messages' => 5,
            ],
            'cors' => [
                'allowed_origins' => ['https://lms.test'],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('default_model', 'runtime-flash')
            ->assertJsonPath('failover.0', 'deepseek-v4-pro')
            ->assertJsonPath('max_steps', 12)
            ->assertJsonPath('compaction.enabled', false)
            ->assertJsonPath('compaction.min_messages', 10)
            ->assertJsonPath('compaction.keep_tail_messages', 5)
            ->assertJsonPath('cors.allowed_origins.0', 'https://lms.test')
            ->assertJsonPath('models.0.id', 'runtime-flash');
    }

    public function test_session_can_update_prompts_without_replacing_models(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'runtime-flash',
                    'label' => 'Runtime Flash',
                    'protocol' => 'responses',
                    'api_model' => 'flash',
                    'url' => 'https://api.example.test',
                    'key' => 'runtime-secret',
                    'thinking' => true,
                    'vision' => false,
                ],
            ],
        ])->assertOk();

        $this->postJson('/veda/admin/settings', [
            'welcome_message' => 'Hello from sidecar.',
            'system_prompt' => 'Always answer in Russian.',
        ])
            ->assertOk()
            ->assertJsonPath('welcome_message', 'Hello from sidecar.')
            ->assertJsonPath('system_prompt', 'Always answer in Russian.')
            ->assertJsonPath('models.0.id', 'runtime-flash');
    }

    public function test_session_can_save_mcp_json_catalog(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'mcp' => [
                'mcpServers' => [
                    'docs' => [
                        'url' => 'https://docs.example.test/mcp',
                        'headers' => [
                            'Authorization' => 'Bearer runtime-mcp-secret',
                        ],
                    ],
                ],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('mcp.mcpServers.docs.url', 'https://docs.example.test/mcp')
            ->assertJsonPath('mcp.mcpServers.docs.headers.Authorization', '••••••••');
    }

    public function test_session_can_save_stdio_mcp_server(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'mcp' => [
                'mcpServers' => [
                    'local' => [
                        'command' => 'php',
                        'args' => ['server.php'],
                        'env' => [
                            'API_KEY' => 'process-secret',
                        ],
                        'envFile' => '${workspaceFolder}/.env',
                        'disabled' => true,
                    ],
                ],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('mcp.mcpServers.local.command', 'php')
            ->assertJsonPath('mcp.mcpServers.local.args.0', 'server.php')
            ->assertJsonPath('mcp.mcpServers.local.env.API_KEY', '••••••••')
            ->assertJsonPath('mcp.mcpServers.local.envFile', '${workspaceFolder}/.env')
            ->assertJsonPath('mcp.mcpServers.local.disabled', true);
    }

    public function test_session_can_save_appearance_without_replacing_models(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'runtime-flash',
                    'label' => 'Runtime Flash',
                    'protocol' => 'responses',
                    'api_model' => 'flash',
                    'url' => 'https://api.example.test',
                    'key' => 'runtime-secret',
                    'thinking' => true,
                    'vision' => false,
                ],
            ],
        ])->assertOk();

        $this->postJson('/veda/admin/settings', [
            'appearance' => [
                'preset' => 'lms',
            ],
        ])
            ->assertOk()
            ->assertJsonPath('appearance.preset', 'lms')
            ->assertJsonPath('appearance.radius', '0px')
            ->assertJsonPath('appearance.tokens.brand', '275 96% 52%')
            ->assertJsonPath('models.0.id', 'runtime-flash');
    }

    public function test_session_can_save_appearance_launcher(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->postJson('/veda/admin/settings', [
            'appearance' => [
                'preset' => 'default',
                'launcher' => [
                    'label' => 'Ask Veda',
                    'icon' => 'rocket',
                ],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('appearance.launcher.label', 'Ask Veda')
            ->assertJsonPath('appearance.launcher.icon', 'rocket');
    }

    public function test_session_can_save_appearance_launcher_image(): void
    {
        $png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';

        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->postJson('/veda/admin/settings', [
            'appearance' => [
                'preset' => 'default',
                'launcher' => [
                    'label' => 'Ask Veda',
                    'icon' => 'rocket',
                    'image' => $png,
                ],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('appearance.launcher.label', 'Ask Veda')
            ->assertJsonPath('appearance.launcher.icon', 'rocket')
            ->assertJsonPath('appearance.launcher.image', $png);
    }

    public function test_unauthenticated_admin_json_visit_is_unauthorized(): void
    {
        $this->getJson('/veda/admin/usage')
            ->assertUnauthorized();
    }

    public function test_authenticated_admin_pages_return_json_payload_for_spa_visits(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $dashboard = $this->getJson('/veda/admin')
            ->assertOk()
            ->assertJsonPath('page', 'dashboard')
            ->assertJsonStructure(['csrf', 'stats', 'chat', 'urls']);

        $this->assertStringContainsString('/veda/admin/usage', (string) $dashboard->json('urls.usage'));
        $this->assertStringContainsString('/veda/admin/appearance', (string) $dashboard->json('urls.appearance'));
        $this->assertStringContainsString('/veda/admin/sources', (string) $dashboard->json('urls.sources'));

        $this->getJson('/veda/admin/usage')
            ->assertOk()
            ->assertJsonPath('page', 'usage')
            ->assertJsonStructure(['usage', 'csrf', 'urls']);

        $this->getJson('/veda/admin/appearance')
            ->assertOk()
            ->assertJsonPath('page', 'appearance');

        $this->getJson('/veda/admin/sources')
            ->assertOk()
            ->assertJsonPath('page', 'sources');

        $this->assertStringContainsString(
            '/veda/admin/code-index/sources',
            (string) $this->getJson('/veda/admin/sources')->json('codeIndex.sources'),
        );
    }
}
