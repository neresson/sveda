<?php

namespace Veda\Laravel\Support;

use Illuminate\Contracts\Auth\Authenticatable;
use Veda\Laravel\Contracts\TokenPolicy;

class NullTokenPolicy implements TokenPolicy
{
    public function assertRequestAllowed(?Authenticatable $user, int $estimatedTokens): void
    {
        //
    }

    public function recordUsage(
        ?Authenticatable $user,
        string $provider,
        string $model,
        int $promptTokens,
        int $completionTokens,
    ): void {
        //
    }
}
