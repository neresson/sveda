<?php

namespace Veda\Laravel\Services;

class UserMessageComposer
{
    public const TAG_USER_REQUEST = 'userRequest';

    public const TAG_CONTEXT = 'context';

    /**
     * @param  array<int, string>  $contextBlocks
     */
    public function compose(string $userRequest, array $contextBlocks = [], bool $systemInitiated = false): string
    {
        $request = trim($userRequest);
        if ($systemInitiated) {
            return $request;
        }

        $parts = [];
        $ambient = array_values(array_filter(array_map('trim', $contextBlocks), fn ($b) => $b !== ''));
        if ($ambient !== []) {
            $parts[] = '<'.self::TAG_CONTEXT.'>';
            $parts[] = implode("\n\n", $ambient);
            $parts[] = '</'.self::TAG_CONTEXT.'>';
            $parts[] = '';
        }

        $parts[] = '<'.self::TAG_USER_REQUEST.'>';
        $parts[] = $request !== '' ? $request : '(empty message)';
        $parts[] = '</'.self::TAG_USER_REQUEST.'>';

        return implode("\n", $parts);
    }

    public function extractUserRequest(string $composed): string
    {
        if (preg_match('/<'.self::TAG_USER_REQUEST.'>\s*([\s\S]*?)\s*<\/'.self::TAG_USER_REQUEST.'>/i', $composed, $m)) {
            return trim($m[1]);
        }

        return trim($composed);
    }
}
