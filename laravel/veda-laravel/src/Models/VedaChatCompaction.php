<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

class VedaChatCompaction extends Model
{
    protected $fillable = [
        'user_id',
        'chat_id',
        'summary_text',
        'transcript_path',
        'source_fingerprint',
        'summarized_at',
    ];

    protected function casts(): array
    {
        return [
            'summarized_at' => 'datetime',
        ];
    }

    public function getTable(): string
    {
        return config('veda.tables.chat_compactions', 'veda_chat_compactions');
    }

    public function user(): BelongsTo
    {
        return $this->belongsTo(config('veda.user_model'), 'user_id');
    }
}
