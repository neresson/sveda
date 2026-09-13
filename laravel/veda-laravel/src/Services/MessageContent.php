<?php

namespace Veda\Laravel\Services;

class MessageContent
{
    public static function toText(mixed $content): string
    {
        if (is_string($content)) {
            return $content;
        }

        if (! is_array($content)) {
            return $content === null ? '' : (string) $content;
        }

        if (array_is_list($content)) {
            $parts = [];
            foreach ($content as $part) {
                if (! is_array($part)) {
                    if (is_string($part) && trim($part) !== '') {
                        $parts[] = trim($part);
                    }

                    continue;
                }

                $type = (string) ($part['type'] ?? '');
                if ($type === 'text') {
                    $text = trim((string) ($part['text'] ?? ''));
                    if ($text !== '') {
                        $parts[] = $text;
                    }

                    continue;
                }

                if ($type === 'image_url') {
                    $parts[] = '[image]';
                }
            }

            return implode("\n", $parts);
        }

        return json_encode($content, JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE) ?: '';
    }
}
