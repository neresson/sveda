<?php

namespace Veda\Laravel\Streaming\Events;

use Laravel\Ai\Streaming\Events\StreamEvent;

class ContextUsage extends StreamEvent
{
    public function __construct(
        public string $id,
        public int $usedTokens,
        public int $maxTokens,
        public float $percent,
        public int $timestamp,
    ) {}

    /**
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        return [
            'id' => $this->id,
            'invocation_id' => $this->invocationId,
            'type' => 'context_usage',
            'used_tokens' => $this->usedTokens,
            'max_tokens' => $this->maxTokens,
            'percent' => $this->percent,
            'timestamp' => $this->timestamp,
        ];
    }

    /**
     * {@inheritdoc}
     */
    public function toVercelProtocolArray(): ?array
    {
        return [
            'type' => 'data-contextUsage',
            'data' => [
                'usedTokens' => $this->usedTokens,
                'maxTokens' => $this->maxTokens,
                'percent' => $this->percent,
            ],
        ];
    }
}
