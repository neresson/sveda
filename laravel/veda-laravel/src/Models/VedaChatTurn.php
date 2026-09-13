<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

class VedaChatTurn extends Model
{
    protected $fillable = [
        'user_id',
        'chat_id',
        'turn_index',
        'frozen_user_content',
        'global_context_rendered',
        'global_context_cache_key',
    ];

    protected function casts(): array
    {
        return [
            'turn_index' => 'integer',
        ];
    }

    public function getTable(): string
    {
        return config('veda.tables.chat_turns', 'veda_chat_turns');
    }

    public function user(): BelongsTo
    {
        return $this->belongsTo(config('veda.user_model'), 'user_id');
    }
}
