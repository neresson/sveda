<?php

namespace Veda\Laravel\Contracts;

use Illuminate\Contracts\Auth\Authenticatable;
use Veda\Laravel\Enums\ToolMode;

interface ToolPermissionHook
{
    public function allows(?Authenticatable $user, string $toolName, ToolMode $mode): bool;
}
