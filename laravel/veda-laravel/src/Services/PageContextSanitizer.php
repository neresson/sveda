<?php

namespace Veda\Laravel\Services;

class PageContextSanitizer
{
    protected const MAX_DEPTH = 6;

    protected const MAX_STRING_LENGTH = 2000;

    protected const MAX_ITEMS = 100;

    protected const MAX_SCREENSHOT_LENGTH = 2_200_000;

    /**
     * @return array<string, mixed>
     */
    public function sanitize(mixed $context): array
    {
        if (! is_array($context)) {
            return [];
        }

        $screenshot = $context['host_screenshot_base64'] ?? null;

        $sanitized = $this->sanitizeValue($context, 0);

        if (! is_array($sanitized)) {
            return [];
        }

        if (is_string($screenshot) && $screenshot !== '') {
            $sanitized['host_screenshot_base64'] = mb_substr($screenshot, 0, self::MAX_SCREENSHOT_LENGTH);
        }

        return $sanitized;
    }

    protected function sanitizeValue(mixed $value, int $depth): mixed
    {
        if ($depth > self::MAX_DEPTH) {
            return null;
        }

        if (is_string($value)) {
            return mb_substr($value, 0, self::MAX_STRING_LENGTH);
        }

        if (is_scalar($value) || $value === null) {
            return $value;
        }

        if (is_array($value)) {
            $out = [];
            $count = 0;
            foreach ($value as $key => $item) {
                if ($count >= self::MAX_ITEMS) {
                    break;
                }
                $out[is_string($key) ? mb_substr($key, 0, 100) : $key] = $this->sanitizeValue($item, $depth + 1);
                $count++;
            }

            return $out;
        }

        return null;
    }
}
