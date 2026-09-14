<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;

class VedaSetting extends Model
{
    public const DOCUMENT_KEY = 'instance';

    protected $fillable = [
        'key',
        'value',
    ];

    protected function casts(): array
    {
        return [
            'value' => 'encrypted:array',
        ];
    }

    public function getTable(): string
    {
        return config('veda.tables.settings', 'veda_settings');
    }
}
