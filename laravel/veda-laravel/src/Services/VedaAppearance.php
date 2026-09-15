<?php

namespace Veda\Laravel\Services;

class VedaAppearance
{
    public const PRESET_DEFAULT = 'default';

    public const PRESET_LMS = 'lms';

    public const PRESET_OCEAN = 'ocean';

    public const PRESET_FOREST = 'forest';

    public const PRESET_SUNSET = 'sunset';

    public const PRESET_SAND = 'sand';

    public const PRESET_ROUNDED = 'rounded';

    public const PRESET_CUSTOM = 'custom';

    public const DEFAULT_RADIUS = '0px';

    public const DEFAULT_LAUNCHER_ICON = 'sparkles';

    public const LAUNCHER_IMAGE_MAX_BYTES = 262144;

    public const LAUNCHER_IMAGE_MAX_CHARS = 360000;

    /**
     * @return list<string>
     */
    public static function launcherIconIds(): array
    {
        return [
            'sparkles',
            'message-circle',
            'message-square',
            'bot',
            'bot-message-square',
            'brain',
            'zap',
            'star',
            'heart',
            'circle-help',
            'rocket',
            'gem',
        ];
    }

    /**
     * @return array{label: string, icon: string, image: string}
     */
    public static function defaultLauncher(): array
    {
        return [
            'label' => '',
            'icon' => self::DEFAULT_LAUNCHER_ICON,
            'image' => '',
        ];
    }

    /**
     * @return list<string>
     */
    public static function presetIds(): array
    {
        return [
            self::PRESET_DEFAULT,
            self::PRESET_LMS,
            self::PRESET_OCEAN,
            self::PRESET_FOREST,
            self::PRESET_SUNSET,
            self::PRESET_SAND,
        ];
    }

    /**
     * @return list<string>
     */
    public static function tokenKeys(): array
    {
        return [
            'background',
            'foreground',
            'card',
            'card_foreground',
            'popover',
            'popover_foreground',
            'primary',
            'primary_foreground',
            'secondary',
            'secondary_foreground',
            'muted',
            'muted_foreground',
            'accent',
            'accent_foreground',
            'destructive',
            'destructive_foreground',
            'border',
            'input',
            'ring',
            'brand',
            'brand_foreground',
        ];
    }

    /**
     * @return array<string, mixed>
     */
    public static function defaults(): array
    {
        return [
            ...self::fromPreset(self::PRESET_DEFAULT),
            'launcher' => self::defaultLauncher(),
        ];
    }

    /**
     * @return array<string, array<string, mixed>>
     */
    public static function presets(): array
    {
        $neutralLight = [
            'background' => '210 20% 98%',
            'foreground' => '0 0% 3.9%',
            'card' => '0 0% 100%',
            'card_foreground' => '0 0% 3.9%',
            'popover' => '0 0% 100%',
            'popover_foreground' => '0 0% 3.9%',
            'primary' => '0 0% 9%',
            'primary_foreground' => '0 0% 98%',
            'secondary' => '0 0% 92.1%',
            'secondary_foreground' => '0 0% 9%',
            'muted' => '0 0% 96.1%',
            'muted_foreground' => '0 0% 45.1%',
            'accent' => '0 0% 96.1%',
            'accent_foreground' => '0 0% 9%',
            'destructive' => '0 84.2% 60.2%',
            'destructive_foreground' => '0 0% 98%',
            'border' => '220 13% 91%',
            'input' => '220 14% 96%',
            'ring' => '0 0% 3.9%',
        ];

        $neutralDark = [
            'background' => '222.2 47.4% 11.2%',
            'foreground' => '210 40% 98%',
            'card' => '222.2 47.4% 14%',
            'card_foreground' => '210 40% 98%',
            'popover' => '222.2 47.4% 14%',
            'popover_foreground' => '210 40% 98%',
            'primary' => '210 40% 98%',
            'primary_foreground' => '222.2 47.4% 11.2%',
            'secondary' => '217.2 32.6% 17.5%',
            'secondary_foreground' => '210 40% 98%',
            'muted' => '217.2 32.6% 17.5%',
            'muted_foreground' => '215 20.2% 65.1%',
            'accent' => '217.2 32.6% 17.5%',
            'accent_foreground' => '210 40% 98%',
            'destructive' => '0 62.8% 30.6%',
            'destructive_foreground' => '210 40% 98%',
            'border' => '217.2 32.6% 17.5%',
            'input' => '217.2 32.6% 17.5%',
            'ring' => '212.7 26.8% 83.9%',
        ];

        return [
            self::PRESET_DEFAULT => self::colorPreset(self::PRESET_DEFAULT, [
                ...$neutralLight,
                'brand' => '0 0% 9%',
                'brand_foreground' => '0 0% 98%',
            ], [
                ...$neutralDark,
                'brand' => '210 40% 98%',
                'brand_foreground' => '222.2 47.4% 11.2%',
            ]),
            self::PRESET_LMS => self::colorPreset(self::PRESET_LMS, [
                ...$neutralLight,
                'accent' => '275 60% 96%',
                'ring' => '275 96% 52%',
                'brand' => '275 96% 52%',
                'brand_foreground' => '0 0% 98%',
            ], [
                ...$neutralDark,
                'accent' => '275 40% 18%',
                'ring' => '275 100% 62%',
                'brand' => '275 100% 42%',
                'brand_foreground' => '0 0% 98%',
            ]),
            self::PRESET_OCEAN => self::colorPreset(self::PRESET_OCEAN, [
                ...$neutralLight,
                'background' => '214 40% 98%',
                'accent' => '214 80% 96%',
                'ring' => '221 83% 53%',
                'brand' => '221 83% 53%',
                'brand_foreground' => '0 0% 98%',
            ], [
                ...$neutralDark,
                'accent' => '217 40% 18%',
                'ring' => '213 94% 68%',
                'brand' => '213 94% 68%',
                'brand_foreground' => '222.2 47.4% 11.2%',
            ]),
            self::PRESET_FOREST => self::colorPreset(self::PRESET_FOREST, [
                ...$neutralLight,
                'background' => '168 25% 98%',
                'card' => '150 20% 99%',
                'popover' => '150 20% 99%',
                'accent' => '166 30% 94%',
                'ring' => '166 72% 32%',
                'brand' => '166 72% 32%',
                'brand_foreground' => '0 0% 98%',
            ], [
                ...$neutralDark,
                'accent' => '166 28% 18%',
                'ring' => '166 50% 52%',
                'brand' => '166 50% 52%',
                'brand_foreground' => '222.2 47.4% 11.2%',
            ]),
            self::PRESET_SUNSET => self::colorPreset(self::PRESET_SUNSET, [
                ...$neutralLight,
                'background' => '28 45% 98%',
                'card' => '30 50% 99%',
                'popover' => '30 50% 99%',
                'accent' => '20 70% 95%',
                'ring' => '16 82% 50%',
                'brand' => '16 82% 50%',
                'brand_foreground' => '0 0% 98%',
            ], [
                ...$neutralDark,
                'accent' => '16 40% 18%',
                'ring' => '18 85% 62%',
                'brand' => '18 85% 62%',
                'brand_foreground' => '222.2 47.4% 11.2%',
            ]),
            self::PRESET_SAND => self::colorPreset(self::PRESET_SAND, [
                ...$neutralLight,
                'background' => '40 33% 97%',
                'card' => '40 40% 99%',
                'popover' => '40 40% 99%',
                'muted' => '36 24% 93%',
                'accent' => '36 30% 93%',
                'border' => '36 18% 86%',
                'input' => '36 22% 94%',
                'ring' => '28 35% 24%',
                'brand' => '28 35% 24%',
                'brand_foreground' => '40 33% 97%',
            ], [
                ...$neutralDark,
                'background' => '30 12% 11%',
                'card' => '30 10% 14%',
                'popover' => '30 10% 14%',
                'accent' => '30 12% 18%',
                'border' => '30 10% 20%',
                'input' => '30 10% 18%',
                'ring' => '36 35% 72%',
                'brand' => '36 35% 72%',
                'brand_foreground' => '30 12% 11%',
            ]),
        ];
    }

    /**
     * @param  array<string, mixed>  $overlay
     * @param  array<string, mixed>|null  $base
     * @return array<string, mixed>
     */
    public static function normalize(mixed $overlay, ?array $base = null): array
    {
        $base = is_array($base) ? $base : [];
        $overlay = is_array($overlay) ? $overlay : [];
        $fallback = self::defaults();

        $preset = is_string($overlay['preset'] ?? null)
            ? $overlay['preset']
            : (is_string($base['preset'] ?? null) ? $base['preset'] : self::PRESET_DEFAULT);
        $legacyRounded = $preset === self::PRESET_ROUNDED;
        if ($legacyRounded) {
            $preset = self::PRESET_FOREST;
        }
        $known = [...self::presetIds(), self::PRESET_CUSTOM];
        if (! in_array($preset, $known, true)) {
            $preset = self::PRESET_DEFAULT;
        }

        if (array_key_exists('radius', $overlay)) {
            $radius = self::sanitizeRadius($overlay['radius']) ?? self::DEFAULT_RADIUS;
        } elseif ($legacyRounded) {
            $radius = '20px';
        } else {
            $radius = self::sanitizeRadius($base['radius'] ?? null) ?? self::DEFAULT_RADIUS;
        }

        $launcher = self::sanitizeLauncher(
            array_key_exists('launcher', $overlay)
                ? array_merge((array) ($base['launcher'] ?? []), is_array($overlay['launcher']) ? $overlay['launcher'] : [])
                : (array) ($base['launcher'] ?? []),
        );

        if ($preset === self::PRESET_CUSTOM) {
            return self::withLauncher([
                'preset' => self::PRESET_CUSTOM,
                'radius' => $radius,
                'tokens' => self::sanitizeTokens(
                    array_merge($fallback['tokens'], (array) ($base['tokens'] ?? []), (array) ($overlay['tokens'] ?? [])),
                    $fallback['tokens'],
                ),
                'dark_tokens' => self::sanitizeTokens(
                    array_merge($fallback['dark_tokens'], (array) ($base['dark_tokens'] ?? []), (array) ($overlay['dark_tokens'] ?? [])),
                    $fallback['dark_tokens'],
                ),
            ], $launcher);
        }

        $fromPreset = self::fromPreset($preset);
        $overlayTokens = is_array($overlay['tokens'] ?? null) ? $overlay['tokens'] : [];
        $overlayDark = is_array($overlay['dark_tokens'] ?? null) ? $overlay['dark_tokens'] : [];
        $tokens = $overlayTokens === []
            ? $fromPreset['tokens']
            : self::sanitizeTokens(array_merge($fromPreset['tokens'], $overlayTokens), $fromPreset['tokens']);
        $darkTokens = $overlayDark === []
            ? $fromPreset['dark_tokens']
            : self::sanitizeTokens(array_merge($fromPreset['dark_tokens'], $overlayDark), $fromPreset['dark_tokens']);
        $customTokens = $tokens !== $fromPreset['tokens'] || $darkTokens !== $fromPreset['dark_tokens'];

        if ($customTokens) {
            return self::withLauncher([
                'preset' => self::PRESET_CUSTOM,
                'radius' => $radius,
                'tokens' => $tokens,
                'dark_tokens' => $darkTokens,
            ], $launcher);
        }

        return self::withLauncher([
            'preset' => $fromPreset['preset'],
            'radius' => $radius,
            'tokens' => $fromPreset['tokens'],
            'dark_tokens' => $fromPreset['dark_tokens'],
        ], $launcher);
    }

    /**
     * @return array<string, mixed>
     */
    public static function fromPreset(string $preset): array
    {
        $presets = self::presets();

        return $presets[$preset] ?? $presets[self::PRESET_DEFAULT];
    }

    /**
     * @param  array<string, string>  $tokens
     * @param  array<string, string>  $darkTokens
     * @return array<string, mixed>
     */
    protected static function colorPreset(string $preset, array $tokens, array $darkTokens): array
    {
        return [
            'preset' => $preset,
            'radius' => self::DEFAULT_RADIUS,
            'tokens' => $tokens,
            'dark_tokens' => $darkTokens,
        ];
    }

    /**
     * @param  array<string, mixed>  $appearance
     * @param  array{label: string, icon: string, image: string}  $launcher
     * @return array<string, mixed>
     */
    protected static function withLauncher(array $appearance, array $launcher): array
    {
        $appearance['launcher'] = $launcher;

        return $appearance;
    }

    /**
     * @param  array<string, mixed>  $launcher
     * @return array{label: string, icon: string, image: string}
     */
    protected static function sanitizeLauncher(array $launcher): array
    {
        $fallback = self::defaultLauncher();
        $label = is_string($launcher['label'] ?? null) ? trim($launcher['label']) : $fallback['label'];
        $label = (string) preg_replace('/\s+/u', ' ', $label);
        if (mb_strlen($label) > 64) {
            $label = mb_substr($label, 0, 64);
        }

        $icon = is_string($launcher['icon'] ?? null) ? trim($launcher['icon']) : $fallback['icon'];
        if (! in_array($icon, self::launcherIconIds(), true)) {
            $icon = $fallback['icon'];
        }

        return [
            'label' => $label,
            'icon' => $icon,
            'image' => self::sanitizeLauncherImage($launcher['image'] ?? ''),
        ];
    }

    protected static function sanitizeLauncherImage(mixed $value): string
    {
        if (! is_string($value)) {
            return '';
        }

        $value = trim($value);
        if ($value === '') {
            return '';
        }

        if (str_starts_with($value, 'data:image/')) {
            return self::sanitizeLauncherImageDataUrl($value);
        }

        if (filter_var($value, FILTER_VALIDATE_URL) !== false) {
            $scheme = strtolower((string) parse_url($value, PHP_URL_SCHEME));
            if (! in_array($scheme, ['http', 'https'], true)) {
                return '';
            }

            return mb_strlen($value) <= 2048 ? $value : '';
        }

        if (str_starts_with($value, '/') && ! str_starts_with($value, '//')) {
            if (str_contains($value, '..') || str_contains($value, '\\') || preg_match('/[\s<>"\']/', $value) === 1) {
                return '';
            }

            return mb_strlen($value) <= 2048 ? $value : '';
        }

        return '';
    }

    protected static function sanitizeLauncherImageDataUrl(string $value): string
    {
        if (strlen($value) > self::LAUNCHER_IMAGE_MAX_CHARS) {
            return '';
        }

        $comma = strpos($value, ',');
        if ($comma === false) {
            return '';
        }

        $meta = strtolower(substr($value, 0, $comma));
        if (preg_match('#^data:image/(png|jpeg|jpg|webp|gif);base64$#', $meta, $matches) !== 1) {
            return '';
        }

        $payload = preg_replace('/\s+/', '', substr($value, $comma + 1)) ?? '';
        if ($payload === '') {
            return '';
        }

        $raw = base64_decode($payload, true);
        if ($raw === false || $raw === '') {
            return '';
        }

        if (strlen($raw) > self::LAUNCHER_IMAGE_MAX_BYTES) {
            return '';
        }

        $info = getimagesizefromstring($raw);
        if ($info === false) {
            return '';
        }

        $detected = match ($info[2] ?? null) {
            IMAGETYPE_PNG => 'png',
            IMAGETYPE_JPEG => 'jpeg',
            IMAGETYPE_GIF => 'gif',
            IMAGETYPE_WEBP => 'webp',
            default => null,
        };
        if ($detected === null) {
            return '';
        }

        $declared = $matches[1] === 'jpg' ? 'jpeg' : $matches[1];
        if ($declared !== $detected) {
            return '';
        }

        return 'data:image/'.$detected.';base64,'.$payload;
    }

    /**
     * @param  array<string, mixed>  $tokens
     * @param  array<string, string>  $fallback
     * @return array<string, string>
     */
    protected static function sanitizeTokens(array $tokens, array $fallback): array
    {
        $clean = [];
        foreach (self::tokenKeys() as $key) {
            $value = self::sanitizeHsl($tokens[$key] ?? null);
            $clean[$key] = $value ?? (string) ($fallback[$key] ?? '');
        }

        return $clean;
    }

    protected static function sanitizeHsl(mixed $value): ?string
    {
        if (! is_string($value)) {
            return null;
        }

        $value = trim($value);
        $value = (string) preg_replace('/^hsla?\(/i', '', $value);
        $value = rtrim($value, ')');
        $value = trim($value);

        if (! preg_match('/^\d{1,3}(?:\.\d+)?\s+\d{1,3}(?:\.\d+)?%\s+\d{1,3}(?:\.\d+)?%$/', $value)) {
            return null;
        }

        return $value;
    }

    protected static function sanitizeRadius(mixed $value): ?string
    {
        if (! is_string($value) && ! is_numeric($value)) {
            return null;
        }

        $value = trim((string) $value);
        if ($value === '0') {
            return '0px';
        }

        if (! preg_match('/^(\d+(?:\.\d+)?)(px|rem|em)$/', $value, $matches)) {
            return null;
        }

        $amount = (float) $matches[1];
        $unit = $matches[2];
        if ($unit === 'px') {
            return ((int) max(0, min(64, $amount))).'px';
        }

        return $matches[1].$unit;
    }
}
