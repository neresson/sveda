<?php

namespace Tests\Unit;

use PHPUnit\Framework\TestCase;

class AdminMotionCssTest extends TestCase
{
    public function test_admin_css_defines_quiet_motion_and_reduced_motion(): void
    {
        $css = file_get_contents(dirname(__DIR__, 2).'/resources/css/admin.css');

        $this->assertNotFalse($css);
        $this->assertStringContainsString('.veda-hover', $css);
        $this->assertStringContainsString('@keyframes veda-grow-y', $css);
        $this->assertStringContainsString('@keyframes veda-grow-x', $css);
        $this->assertStringContainsString('.veda-drawer-enter-from .veda-drawer-panel', $css);
        $this->assertStringContainsString('prefers-reduced-motion: reduce', $css);
    }

    public function test_admin_shell_caps_desktop_width(): void
    {
        $vue = file_get_contents(dirname(__DIR__, 2).'/resources/js/admin/AdminShell.vue');

        $this->assertNotFalse($vue);
        $this->assertStringContainsString('max-w-7xl', $vue);
        $this->assertStringContainsString('mx-auto', $vue);
    }

    public function test_admin_css_uses_thin_scrollbars(): void
    {
        $css = file_get_contents(dirname(__DIR__, 2).'/resources/css/admin.css');

        $this->assertNotFalse($css);
        $this->assertStringContainsString('scrollbar-width: thin', $css);
        $this->assertStringContainsString('::-webkit-scrollbar', $css);
        $this->assertStringContainsString('scrollbar-gutter: stable', $css);
    }

    public function test_admin_css_imports_veda_theme_tokens_instead_of_host_dumps(): void
    {
        $css = file_get_contents(dirname(__DIR__, 2).'/resources/css/admin.css');
        $chat = file_get_contents(dirname(__DIR__, 2).'/resources/js/admin/AdminVedaChat.vue');
        $theme = file_get_contents(dirname(__DIR__, 4).'/packages/vue/src/theme.css');

        $this->assertNotFalse($css);
        $this->assertNotFalse($chat);
        $this->assertNotFalse($theme);
        $this->assertStringContainsString('packages/vue/src/theme.css', $css);
        $this->assertStringContainsString('--color-background: hsl(var(--veda-background, var(--background)))', $css);
        $this->assertStringContainsString('--color-muted: #6f6b63', $css);
        $this->assertStringNotContainsString('--background: 210 20% 98%', $css);
        $this->assertStringContainsString('class="veda-chat veda-chat-host"', $chat);
        $this->assertStringContainsString(':where(.veda-chat)', $theme);
        $this->assertStringContainsString('--veda-background:', $theme);
        $this->assertStringContainsString('--veda-radius: 0;', $theme);
        $this->assertStringNotContainsString('--background: var(--veda-background)', $theme);
    }
}
