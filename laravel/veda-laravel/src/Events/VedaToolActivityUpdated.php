<?php

namespace Veda\Laravel\Events;

use Illuminate\Broadcasting\InteractsWithSockets;
use Illuminate\Broadcasting\PrivateChannel;
use Illuminate\Contracts\Broadcasting\ShouldBroadcastNow;
use Illuminate\Foundation\Events\Dispatchable;

class VedaToolActivityUpdated implements ShouldBroadcastNow
{
    use Dispatchable, InteractsWithSockets;

    /**
     * @param  array<int, array{id: string, label: string, status: string, detail?: string}>  $tasks
     */
    public function __construct(
        public readonly int|string $userId,
        public readonly string $chatId,
        public readonly array $tasks,
        public readonly ?string $runId = null,
    ) {}

    public function broadcastOn(): PrivateChannel
    {
        $prefix = (string) config('veda.broadcasting.channel_prefix', 'veda');

        return new PrivateChannel($prefix.'.'.$this->userId.'.'.$this->chatId);
    }

    public function broadcastAs(): string
    {
        return 'veda.tool.activity';
    }

    /**
     * @return array{tasks: array<int, array<string, mixed>>, run_id: string|null}
     */
    public function broadcastWith(): array
    {
        return [
            'tasks' => $this->tasks,
            'run_id' => $this->runId,
        ];
    }
}
