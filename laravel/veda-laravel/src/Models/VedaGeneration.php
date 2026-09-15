<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Laravel\Ai\Responses\Data\Usage;

class VedaGeneration extends Model
{
    public const STATUS_PENDING = 'pending';

    public const STATUS_COMPLETED = 'completed';

    public const STATUS_FAILED = 'failed';

    protected $fillable = [
        'user_id',
        'generation_type',
        'model',
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

    /**
     * @return array{prompt_tokens: int, completion_tokens: int, tokens_used: int}
     */
    public static function tokensFromUsage(Usage $usage): array
    {
        $prompt = max(0, $usage->promptTokens) + max(0, $usage->cacheReadInputTokens);
        $completion = max(0, $usage->completionTokens);

        return [
            'prompt_tokens' => $prompt,
            'completion_tokens' => $completion,
            'tokens_used' => $prompt + $completion,
        ];
    }

    public function markAsCompleted(
        array $content,
        ?int $tokensUsed = null,
        ?int $promptTokens = null,
        ?int $completionTokens = null,
    ): void {
        $this->generated_content = $content;
        $this->tokens_used = $tokensUsed;
        $this->prompt_tokens = $promptTokens;
        $this->completion_tokens = $completionTokens;
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
