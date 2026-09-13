<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

class VedaChatHistory extends Model
{
    protected $fillable = [
        'user_id',
        'visitor_id',
        'chat_id',
        'title',
        'messages',
        'conversation_history',
        'tokens_used',
        'version',
        'organization_id',
        'created_at',
        'updated_at',
    ];

    protected function casts(): array
    {
        return [
            'messages' => 'array',
            'conversation_history' => 'array',
            'tokens_used' => 'integer',
            'version' => 'integer',
            'created_at' => 'datetime',
            'updated_at' => 'datetime',
        ];
    }

    public function getTable(): string
    {
        return config('veda.tables.chat_histories', 'veda_chat_histories');
    }

    public function user(): BelongsTo
    {
        return $this->belongsTo(config('veda.user_model'), 'user_id');
    }
}
