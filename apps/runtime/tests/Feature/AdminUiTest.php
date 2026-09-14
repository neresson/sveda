<?php

namespace Tests\Feature;

use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

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
            ->assertSee('veda\\/admin\\/login', false);
    }

    public function test_login_rejects_wrong_key(): void
    {
        $this->from('/veda/admin')->post('/veda/admin/login', [
            'key' => 'wrong',
        ])->assertRedirect('/veda/admin');
    }

    public function test_runtime_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"runtime"', false)
            ->assertSee('veda\\/admin\\/models', false)
            ->assertSee('veda\\/admin\\/prompts', false)
            ->assertSee('/veda/admin/settings', false);
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
}
