<?php

namespace Veda\Laravel\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Veda\Laravel\Services\VedaAppearance;

class VedaAppearanceTest extends TestCase
{
    public function test_defaults_use_square_veda_preset(): void
    {
        $appearance = VedaAppearance::defaults();

        $this->assertSame(VedaAppearance::PRESET_DEFAULT, $appearance['preset']);
        $this->assertSame('0px', $appearance['radius']);
        $this->assertSame('0 0% 9%', $appearance['tokens']['brand']);
        $this->assertSame('210 40% 98%', $appearance['dark_tokens']['brand']);
        $this->assertSame('', $appearance['launcher']['label']);
        $this->assertSame('sparkles', $appearance['launcher']['icon']);
        $this->assertSame('', $appearance['launcher']['image']);
    }

    public function test_named_preset_replaces_tokens(): void
    {
        $appearance = VedaAppearance::normalize(['preset' => 'lms']);

        $this->assertSame('lms', $appearance['preset']);
        $this->assertSame('0px', $appearance['radius']);
        $this->assertSame('275 96% 52%', $appearance['tokens']['brand']);
        $this->assertSame('275 100% 42%', $appearance['dark_tokens']['brand']);
    }

    public function test_named_preset_keeps_independent_radius(): void
    {
        $appearance = VedaAppearance::normalize([
            'preset' => 'ocean',
            'radius' => '12px',
        ]);

        $this->assertSame('ocean', $appearance['preset']);
        $this->assertSame('12px', $appearance['radius']);
        $this->assertSame('221 83% 53%', $appearance['tokens']['brand']);
    }

    public function test_legacy_rounded_preset_maps_to_forest(): void
    {
        $appearance = VedaAppearance::normalize(['preset' => 'rounded'], VedaAppearance::defaults());

        $this->assertSame('forest', $appearance['preset']);
        $this->assertSame('20px', $appearance['radius']);
        $this->assertSame('166 72% 32%', $appearance['tokens']['brand']);
    }

    public function test_token_overrides_promote_named_preset_to_custom(): void
    {
        $appearance = VedaAppearance::normalize([
            'preset' => 'lms',
            'tokens' => [
                'brand' => '12 80% 50%',
            ],
        ]);

        $this->assertSame('custom', $appearance['preset']);
        $this->assertSame('0px', $appearance['radius']);
        $this->assertSame('12 80% 50%', $appearance['tokens']['brand']);
        $this->assertSame('275 96% 52%', VedaAppearance::fromPreset('lms')['tokens']['brand']);
    }

    public function test_custom_keeps_manual_radius_and_rejects_invalid_hsl(): void
    {
        $appearance = VedaAppearance::normalize([
            'preset' => 'custom',
            'radius' => '12px',
            'tokens' => [
                'brand' => 'not-a-color',
                'background' => 'hsl(210 20% 98%)',
            ],
        ]);

        $this->assertSame('custom', $appearance['preset']);
        $this->assertSame('12px', $appearance['radius']);
        $this->assertSame('210 20% 98%', $appearance['tokens']['background']);
        $this->assertSame(VedaAppearance::defaults()['tokens']['brand'], $appearance['tokens']['brand']);
    }

    public function test_parameter_token_overrides_become_custom(): void
    {
        $appearance = VedaAppearance::normalize([
            'preset' => 'lms',
            'radius' => '12px',
            'tokens' => [
                'brand' => 'hsl(12 80% 50%)',
            ],
        ]);

        $this->assertSame('custom', $appearance['preset']);
        $this->assertSame('12px', $appearance['radius']);
        $this->assertSame('12 80% 50%', $appearance['tokens']['brand']);
        $this->assertSame('275 96% 52%', VedaAppearance::fromPreset('lms')['tokens']['brand']);
    }

    public function test_unknown_preset_falls_back_to_default(): void
    {
        $appearance = VedaAppearance::normalize(['preset' => 'neon']);

        $this->assertSame('default', $appearance['preset']);
        $this->assertSame('0px', $appearance['radius']);
    }

    public function test_all_color_presets_are_registered(): void
    {
        $this->assertSame(
            ['default', 'lms', 'ocean', 'forest', 'sunset', 'sand'],
            VedaAppearance::presetIds(),
        );
        $this->assertSame('16 82% 50%', VedaAppearance::fromPreset('sunset')['tokens']['brand']);
        $this->assertSame('28 35% 24%', VedaAppearance::fromPreset('sand')['tokens']['brand']);
    }

    public function test_named_presets_include_dark_tokens(): void
    {
        foreach (VedaAppearance::presetIds() as $preset) {
            $appearance = VedaAppearance::fromPreset($preset);

            $this->assertNotSame(
                $appearance['tokens']['background'],
                $appearance['dark_tokens']['background'],
            );
            $this->assertNotSame('', $appearance['dark_tokens']['brand']);
        }
    }

    public function test_theme_parameter_is_not_persisted(): void
    {
        $appearance = VedaAppearance::normalize([
            'preset' => 'lms',
            'theme' => 'dark',
            'radius' => '8px',
        ]);

        $this->assertSame('lms', $appearance['preset']);
        $this->assertSame('8px', $appearance['radius']);
        $this->assertSame('275 100% 42%', $appearance['dark_tokens']['brand']);
        $this->assertArrayNotHasKey('theme', $appearance);
    }

    public function test_launcher_is_independent_of_color_preset(): void
    {
        $appearance = VedaAppearance::normalize(
            ['preset' => 'ocean'],
            [
                'preset' => 'default',
                'launcher' => [
                    'label' => 'Help',
                    'icon' => 'bot',
                ],
            ],
        );

        $this->assertSame('ocean', $appearance['preset']);
        $this->assertSame('Help', $appearance['launcher']['label']);
        $this->assertSame('bot', $appearance['launcher']['icon']);
        $this->assertSame('', $appearance['launcher']['image']);
    }

    public function test_launcher_overlay_is_sanitized(): void
    {
        $appearance = VedaAppearance::normalize([
            'preset' => 'lms',
            'launcher' => [
                'label' => "  Ask   Veda  \nnow  ",
                'icon' => 'not-an-icon',
            ],
        ]);

        $this->assertSame('Ask Veda now', $appearance['launcher']['label']);
        $this->assertSame('sparkles', $appearance['launcher']['icon']);
        $this->assertSame('', $appearance['launcher']['image']);
    }

    public function test_saved_launcher_is_kept(): void
    {
        $appearance = VedaAppearance::normalize([
            'preset' => 'forest',
            'radius' => '20px',
            'launcher' => [
                'label' => 'Спросить',
                'icon' => 'rocket',
            ],
        ]);

        $this->assertSame('forest', $appearance['preset']);
        $this->assertSame('Спросить', $appearance['launcher']['label']);
        $this->assertSame('rocket', $appearance['launcher']['icon']);
        $this->assertSame('', $appearance['launcher']['image']);
    }

    public function test_launcher_image_data_url_is_kept(): void
    {
        $png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
        $appearance = VedaAppearance::normalize([
            'preset' => 'ocean',
            'launcher' => [
                'image' => $png,
            ],
        ]);

        $this->assertSame('ocean', $appearance['preset']);
        $this->assertSame($png, $appearance['launcher']['image']);
        $this->assertSame('sparkles', $appearance['launcher']['icon']);
    }

    public function test_launcher_image_rejects_svg_scripts_and_protocol_relative_urls(): void
    {
        $svg = 'data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciLz4=';

        $this->assertSame('', VedaAppearance::normalize(['launcher' => ['image' => $svg]])['launcher']['image']);
        $this->assertSame('', VedaAppearance::normalize(['launcher' => ['image' => 'javascript:alert(1)']])['launcher']['image']);
        $this->assertSame('', VedaAppearance::normalize(['launcher' => ['image' => '//evil.example/x.png']])['launcher']['image']);
        $this->assertSame('', VedaAppearance::normalize(['launcher' => ['image' => 'data:image/png;base64,'.str_repeat('A', 400000)]])['launcher']['image']);
    }

    public function test_launcher_image_allows_https_and_relative_urls(): void
    {
        $this->assertSame(
            'https://cdn.example.com/veda.png',
            VedaAppearance::normalize(['launcher' => ['image' => 'https://cdn.example.com/veda.png']])['launcher']['image'],
        );
        $this->assertSame(
            '/icons/veda.png',
            VedaAppearance::normalize(['launcher' => ['image' => '/icons/veda.png']])['launcher']['image'],
        );
    }

    public function test_launcher_image_is_independent_of_color_preset(): void
    {
        $png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
        $appearance = VedaAppearance::normalize(
            ['preset' => 'sunset'],
            [
                'preset' => 'default',
                'launcher' => [
                    'label' => 'Help',
                    'icon' => 'bot',
                    'image' => $png,
                ],
            ],
        );

        $this->assertSame('sunset', $appearance['preset']);
        $this->assertSame('Help', $appearance['launcher']['label']);
        $this->assertSame('bot', $appearance['launcher']['icon']);
        $this->assertSame($png, $appearance['launcher']['image']);
    }
}
