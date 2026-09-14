<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Foundation\Http\Middleware\ValidateCsrfToken;
use Veda\Laravel\Tests\TestCase;

class AdminUiTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.admin.api_key', 'veda-admin-secret');
    }

    protected function setUp(): void
    {
        parent::setUp();

        $this->withoutMiddleware(ValidateCsrfToken::class);
    }

    public function test_admin_page_shows_setup_when_admin_key_is_unconfigured(): void
    {
        config()->set('veda.admin.api_key', '');

        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('name="key"', false);
    }

    public function test_admin_setup_creates_key_and_opens_settings(): void
    {
        config()->set('veda.admin.api_key', '');

        $this->post('/veda/admin/setup', [
            'key' => 'veda-admin-secret-from-setup',
            'key_confirmation' => 'veda-admin-secret-from-setup',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('name="default_model"', false);
    }

    public function test_admin_page_shows_login_when_unauthenticated(): void
    {
        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('Veda');
    }

    public function test_admin_login_rejects_wrong_key(): void
    {
        $this->from('/veda/admin')->post('/veda/admin/login', [
            'key' => 'wrong',
        ])->assertRedirect('/veda/admin');
    }

    public function test_admin_login_and_settings_form(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('name="default_model"', false)
            ->assertSee('name="system_prompt"', false)
            ->assertSee('name="welcome_message"', false);
    }

    public function test_session_json_can_add_a_model(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'custom-flash',
                    'label' => 'Custom Flash',
                    'protocol' => 'responses',
                    'api_model' => 'flash',
                    'url' => 'https://api.example.test',
                    'key' => 'model-secret',
                    'thinking' => true,
                    'vision' => false,
                    'aliases' => ['flash'],
                ],
            ],
        ])
            ->assertOk()
            ->assertJsonPath('models.0.id', 'custom-flash')
            ->assertJsonPath('models.0.label', 'Custom Flash')
            ->assertJsonPath('models.0.key', '••••••••');
    }

    public function test_session_json_can_update_an_existing_model_without_replacing_the_key(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'custom-flash',
                    'label' => 'Custom Flash',
                    'protocol' => 'responses',
                    'api_model' => 'flash',
                    'url' => 'https://api.example.test',
                    'key' => 'model-secret',
                    'thinking' => true,
                    'vision' => false,
                    'aliases' => ['flash'],
                ],
            ],
        ])->assertOk();

        $this->postJson('/veda/admin/settings', [
            'models' => [
                [
                    'id' => 'custom-flash',
                    'label' => 'Custom Flash Updated',
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
            ->assertJsonPath('models.0.id', 'custom-flash')
            ->assertJsonPath('models.0.label', 'Custom Flash Updated')
            ->assertJsonPath('models.0.protocol', 'anthropic')
            ->assertJsonPath('models.0.key', '••••••••');
    }

    public function test_session_json_can_update_runtime_settings(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'default_model' => 'deepseek-v4-pro',
            'failover' => ['deepseek-v4-flash-anthropic'],
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
            ->assertJsonPath('default_model', 'deepseek-v4-pro')
            ->assertJsonPath('failover.0', 'deepseek-v4-flash-anthropic')
            ->assertJsonPath('max_steps', 12)
            ->assertJsonPath('compaction.enabled', false)
            ->assertJsonPath('cors.allowed_origins.0', 'https://lms.test')
            ->assertJsonPath('models.0.id', 'deepseek-v4-flash-responses');
    }

    public function test_session_json_can_update_prompts(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ]);

        $this->postJson('/veda/admin/settings', [
            'welcome_message' => 'Hello from sidecar.',
            'system_prompt' => 'Always answer in Russian.',
        ])
            ->assertOk()
            ->assertJsonPath('welcome_message', 'Hello from sidecar.')
            ->assertJsonPath('system_prompt', 'Always answer in Russian.')
            ->assertJsonPath('models.0.id', 'deepseek-v4-flash-responses');
    }
}
