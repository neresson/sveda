<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Contracts\JsonSchema\JsonSchema;
use Stringable;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Services\SubagentRunner;

class SpawnParallelTasksTool extends VedaTool
{
    public function __construct(?Authenticatable $user = null)
    {
        parent::__construct($user);
    }

    public function name(): string
    {
        return 'spawn_tasks';
    }

    public function description(): Stringable|string
    {
        return 'Run multiple independent background tasks in parallel (e.g. searching several sources at once). Each task has a type, an input payload and an optional label. Returns a combined result for all tasks. Use this instead of sequential calls when tasks are independent.';
    }

    public function mode(): ToolMode
    {
        return ToolMode::Read;
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'tasks' => $schema->array()
                ->items($schema->object([
                    'type' => $schema->string()->description('Task type registered by the host application.')->required(),
                    'input' => $schema->object()->description('Input payload for the task.')->required(),
                    'label' => $schema->string()->description('Short human-readable label for progress display.'),
                ]))
                ->description('List of independent tasks to run in parallel.')
                ->required(),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        $tasks = $arguments['tasks'] ?? [];

        if (! is_array($tasks) || $tasks === []) {
            return ['success' => false, 'error' => 'At least one task is required.'];
        }

        $normalized = [];
        foreach ($tasks as $task) {
            if (! is_array($task) || ! isset($task['type']) || ! is_string($task['type'])) {
                return ['success' => false, 'error' => 'Each task must have a string "type".'];
            }

            $normalized[] = [
                'type' => $task['type'],
                'input' => is_array($task['input'] ?? null) ? $task['input'] : [],
                'label' => isset($task['label']) && is_string($task['label']) ? $task['label'] : null,
            ];
        }

        $result = app(SubagentRunner::class)->run($normalized, $this->user, RequestContext::current());

        if (! ($result['success'] ?? false)) {
            return ['success' => false, 'error' => $result['error'] ?? 'Failed to run tasks.'];
        }

        return ['success' => true, 'data' => ['tasks' => $result['tasks']]];
    }
}
