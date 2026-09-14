<?php

namespace Veda\Laravel\Enums;

enum VedaProtocol: string
{
    case Responses = 'responses';
    case Anthropic = 'anthropic';

    public function driver(): string
    {
        return match ($this) {
            self::Responses => 'veda-responses',
            self::Anthropic => 'veda-anthropic',
        };
    }

    public static function tryFromMixed(mixed $value): ?self
    {
        if ($value instanceof self) {
            return $value;
        }

        if (! is_string($value) || $value === '') {
            return null;
        }

        return self::tryFrom(strtolower(trim($value)));
    }
}
