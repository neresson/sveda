<?php

namespace Veda\Laravel\Gateway\Concerns;

use Veda\Laravel\Services\RequestContext;

trait AttachesHostScreenshot
{
    /**
     * @param  array<int, array<string, mixed>>  $chatMessages
     * @return array<int, array<string, mixed>>
     */
    protected function attachHostScreenshot(array $chatMessages, string $providerName, string $model): array
    {
        if (! $this->supportsHostScreenshot($providerName, $model)) {
            return $chatMessages;
        }

        $pageContext = RequestContext::current()?->pageContext ?? [];
        $mime = $pageContext['host_screenshot_mime'] ?? null;
        $base64 = $pageContext['host_screenshot_base64'] ?? null;

        if (! is_string($mime) || $mime === '' || ! is_string($base64) || strlen($base64) < 32) {
            return $chatMessages;
        }

        $lastUserIndex = null;
        for ($index = count($chatMessages) - 1; $index >= 0; $index--) {
            if (($chatMessages[$index]['role'] ?? '') === 'user') {
                $lastUserIndex = $index;
                break;
            }
        }

        if ($lastUserIndex === null) {
            return $chatMessages;
        }

        $content = $chatMessages[$lastUserIndex]['content'] ?? '';
        if (is_array($content)) {
            foreach ($content as $part) {
                if (is_array($part) && ($part['type'] ?? '') === 'image_url') {
                    return $chatMessages;
                }
            }

            $chatMessages[$lastUserIndex]['content'][] = $this->hostScreenshotImagePart($mime, $base64);

            return $chatMessages;
        }

        $text = trim((string) $content);
        $chatMessages[$lastUserIndex]['content'] = [
            [
                'type' => 'text',
                'text' => $text !== '' ? $text : 'Help me with this host application screen.',
            ],
            $this->hostScreenshotImagePart($mime, $base64),
        ];

        return $chatMessages;
    }

    protected function supportsHostScreenshot(string $providerName, string $model): bool
    {
        $markers = config('veda.vision_markers', ['yandex', 'timeweb', 'gemini', 'qwen']);
        if (! is_array($markers)) {
            return false;
        }

        $haystack = strtolower($providerName.'/'.$model);
        foreach ($markers as $marker) {
            if (is_string($marker) && $marker !== '' && str_contains($haystack, strtolower($marker))) {
                return true;
            }
        }

        return false;
    }

    /**
     * @return array<string, mixed>
     */
    protected function hostScreenshotImagePart(string $mime, string $base64): array
    {
        return [
            'type' => 'image_url',
            'image_url' => [
                'url' => 'data:'.$mime.';base64,'.$base64,
                'detail' => 'low',
            ],
        ];
    }
}
