<?php

namespace Veda\Laravel\Streaming\Events;

use Laravel\Ai\Streaming\Events\StreamEvent;

class MaxStepsReached extends StreamEvent
{
    public function __construct(
        public string $id,
        public int $maxSteps,
        public int $depth,
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
            'type' => 'max_steps_reached',
            'max_steps' => $this->maxSteps,
            'depth' => $this->depth,
            'timestamp' => $this->timestamp,
        ];
    }

    /**
     * {@inheritdoc}
     */
    public function toVercelProtocolArray(): ?array
    {
        return [
            'type' => 'data-maxStepsReached',
            'data' => [
                'maxSteps' => $this->maxSteps,
                'canContinue' => true,
            ],
        ];
    }
}
