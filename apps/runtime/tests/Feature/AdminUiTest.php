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

    public function test_models_page_renders_after_login(): void
    {
        $this->post('/veda/admin/login', [
            'key' => 'veda-admin-secret',
        ])->assertRedirect('/veda/admin');

        $this->get('/veda/admin')
            ->assertOk()
            ->assertSee('id="veda-admin"', false)
            ->assertSee('"page":"models"', false)
            ->assertSee('/veda/admin/settings', false);
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
}
