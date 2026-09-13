<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

class VedaGeneration extends Model
{
    public const STATUS_PENDING = 'pending';

    public const STATUS_COMPLETED = 'completed';

    public const STATUS_FAILED = 'failed';

    protected $fillable = [
        'user_id',
        'generation_type',
        'prompt',
        'generated_content',
        'status',
        'error_message',
        'entity_id',
        'entity_type',
        'tokens_used',
        'prompt_tokens',
        'completion_tokens',
    ];

    protected function casts(): array
    {
        return [
            'generated_content' => 'array',
            'tokens_used' => 'integer',
            'prompt_tokens' => 'integer',
            'completion_tokens' => 'integer',
        ];
    }

    public function getTable(): string
    {
        return config('veda.tables.generations', 'veda_generations');
    }

    public function user(): BelongsTo
    {
        return $this->belongsTo(config('veda.user_model'), 'user_id');
    }

    public function isPending(): bool
    {
        return $this->status === self::STATUS_PENDING;
    }

    public function isCompleted(): bool
    {
        return $this->status === self::STATUS_COMPLETED;
    }

    public function isFailed(): bool
    {
        return $this->status === self::STATUS_FAILED;
    }

    public function markAsCompleted(array $content, ?int $tokensUsed = null): void
    {
        $this->generated_content = $content;
        $this->tokens_used = $tokensUsed;
        $this->status = self::STATUS_COMPLETED;
        $this->save();
    }

    public function markAsFailed(string $errorMessage): void
    {
        $this->error_message = $errorMessage;
        $this->status = self::STATUS_FAILED;
        $this->save();
    }
}
