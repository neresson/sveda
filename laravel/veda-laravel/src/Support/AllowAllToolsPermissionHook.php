<?php

namespace Veda\Laravel\Support;

use Illuminate\Contracts\Auth\Authenticatable;
use Veda\Laravel\Contracts\ToolPermissionHook;
use Veda\Laravel\Enums\ToolMode;

class AllowAllToolsPermissionHook implements ToolPermissionHook
{
    public function allows(?Authenticatable $user, string $toolName, ToolMode $mode): bool
    {
        return true;
    }
}
