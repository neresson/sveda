<?php

namespace Veda\Laravel\Listeners;

use Illuminate\Support\Str;
use Laravel\Ai\Events\InvokingTool;
use Laravel\Ai\Events\ToolInvoked;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Services\ToolActivityBroadcaster;
use Veda\Laravel\VedaManager;

class BroadcastVedaToolActivity
{
    public function __construct(
        protected ToolActivityBroadcaster $broadcaster,
    ) {}

    public function handle(InvokingTool|ToolInvoked $event): void
    {
        $context = RequestContext::current();

        if ($context === null || $context->userId === null || $context->chatId === null) {
            return;
        }

        $toolName = $this->resolveToolName($event);

        if ($toolName === null) {
            return;
        }

        $status = $event instanceof InvokingTool ? 'running' : 'completed';

        $this->broadcaster->broadcast(
            $context->userId,
            $context->chatId,
            [[
                'id' => $event->toolInvocationId ?? $toolName,
                'label' => $this->labelFor($toolName),
                'status' => $status,
            ]],
            $context->runId,
        );
    }

    protected function resolveToolName(InvokingTool|ToolInvoked $event): ?string
    {
        $tool = $event->tool ?? null;

        if (is_object($tool) && method_exists($tool, 'name')) {
            return (string) $tool->name();
        }

        if (is_object($tool)) {
            return Str::snake(class_basename($tool));
        }

        return null;
    }

    protected function labelFor(string $toolName): string
    {
        $label = app(VedaManager::class)->labelForSubagentType($toolName);

        return $label !== $toolName ? $label : str_replace('_', ' ', $toolName);
    }
}
