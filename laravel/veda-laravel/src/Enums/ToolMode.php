<?php

namespace Veda\Laravel\Enums;

enum ToolMode: string
{
    case Read = 'read';
    case Write = 'write';
}
