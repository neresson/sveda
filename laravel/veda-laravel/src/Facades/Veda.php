<?php

namespace Veda\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

class Veda extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return 'veda';
    }
}
