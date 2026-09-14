<?php

namespace Veda\Laravel\Gateway\Concerns;

use Laravel\Ai\Providers\Provider;
use Veda\Laravel\Services\RequestContext;

trait AttachesHostScreenshot
{
    protected function hasHostScreenshot(): bool
    {
        return $this->hostScreenshotParts() !== null;
    }

    /**
     * @param  array<int, array<string, mixed>>  $input
     */
    protected function appendResponsesScreenshot(array &$input, ?Provider $provider): void
    {
        if (! $this->providerSupportsVision($provider)) {
            return;
        }

        $screenshot = $this->hostScreenshotParts();
        if ($screenshot === null) {
            return;
        }

        $lastUserIndex = $this->lastRoleIndex($input, 'user');
        if ($lastUserIndex === null) {
            return;
        }

        $content = $input[$lastUserIndex]['content'] ?? [];
        if (! is_array($content)) {
            return;
        }

        foreach ($content as $part) {
            if (is_array($part) && ($part['type'] ?? '') === 'input_image') {
                return;
            }
        }

        $input[$lastUserIndex]['content'][] = [
            'type' => 'input_image',
            'image_url' => 'data:'.$screenshot['mime'].';base64,'.$screenshot['base64'],
        ];
    }

    /**
     * @param  array<int, array<string, mixed>>  $mapped
     */
    protected function appendAnthropicScreenshot(array &$mapped, ?Provider $provider): void
    {
        if (! $this->providerSupportsVision($provider)) {
            return;
        }

        $screenshot = $this->hostScreenshotParts();
        if ($screenshot === null) {
            return;
        }

        $lastUserIndex = $this->lastRoleIndex($mapped, 'user');
        if ($lastUserIndex === null) {
            return;
        }

        $content = $mapped[$lastUserIndex]['content'] ?? [];
        if (! is_array($content)) {
            return;
        }

        foreach ($content as $part) {
            if (is_array($part) && ($part['type'] ?? '') === 'image') {
                return;
            }
        }

        $mapped[$lastUserIndex]['content'][] = [
            'type' => 'image',
            'source' => [
                'type' => 'base64',
                'media_type' => $screenshot['mime'],
                'data' => $screenshot['base64'],
            ],
        ];
    }

    protected function providerSupportsVision(?Provider $provider): bool
    {
        if ($provider === null) {
            return false;
        }

        return filter_var(
            $provider->additionalConfiguration()['vision'] ?? false,
            FILTER_VALIDATE_BOOL
        ) === true;
    }

    /**
     * @return array{mime: string, base64: string}|null
     */
    protected function hostScreenshotParts(): ?array
    {
        $pageContext = RequestContext::current()?->pageContext ?? [];
        $mime = $pageContext['host_screenshot_mime'] ?? null;
        $base64 = $pageContext['host_screenshot_base64'] ?? null;

        if (! is_string($mime) || $mime === '' || ! is_string($base64) || strlen($base64) < 32) {
            return null;
        }

        return [
            'mime' => $mime,
            'base64' => $base64,
        ];
    }

    /**
     * @param  array<int, array<string, mixed>>  $items
     */
    protected function lastRoleIndex(array $items, string $role): ?int
    {
        for ($index = count($items) - 1; $index >= 0; $index--) {
            if (($items[$index]['role'] ?? '') === $role) {
                return $index;
            }
        }

        return null;
    }
}
