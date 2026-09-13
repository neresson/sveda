<?php

namespace Veda\Laravel\Contracts;

use Illuminate\Contracts\Auth\Authenticatable;
use Veda\Laravel\Exceptions\VedaTokenLimitExceededException;

interface TokenPolicy
{
    /**
     * @throws VedaTokenLimitExceededException
     */
    public function assertRequestAllowed(?Authenticatable $user, int $estimatedTokens): void;

    public function recordUsage(
        ?Authenticatable $user,
        string $provider,
        string $model,
        int $promptTokens,
        int $completionTokens,
    ): void;
}
