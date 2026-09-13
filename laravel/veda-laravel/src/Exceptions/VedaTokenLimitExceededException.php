<?php

namespace Veda\Laravel\Exceptions;

use RuntimeException;

class VedaTokenLimitExceededException extends RuntimeException
{
    public function __construct(
        string $message = 'AI token limit exceeded.',
        public readonly string $period = 'month',
        public readonly string $mode = 'shared',
    ) {
        parent::__construct($message);
    }
}
