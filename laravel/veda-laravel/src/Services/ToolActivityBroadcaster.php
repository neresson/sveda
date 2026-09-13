<?php

namespace Veda\Laravel\Services;

use Veda\Laravel\Events\VedaToolActivityUpdated;

class ToolActivityBroadcaster
{
    /**
     * @param  array<int, array{id: string, label: string, status: string, detail?: string}>  $tasks
     */
    public function broadcast(int|string $userId, string $chatId, array $tasks, ?string $runId = null): void
    {
        if (! (bool) config('veda.broadcasting.enabled', true)) {
            return;
        }

        if ($chatId === '') {
            return;
        }

        try {
            VedaToolActivityUpdated::dispatch($userId, $chatId, $tasks, $runId);
        } catch (\Throwable) {
        }
    }

    /**
     * @param  array<int, array{id: string, label: string, status: string, detail?: string}>  $tasks
     */
    public function broadcastForContext(?RequestContext $context, array $tasks): void
    {
        if ($context === null || $context->userId === null || $context->chatId === null) {
            return;
        }

        $this->broadcast($context->userId, $context->chatId, $tasks, $context->runId);
    }
}
