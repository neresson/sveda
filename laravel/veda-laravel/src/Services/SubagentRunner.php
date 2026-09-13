<?php

namespace Veda\Laravel\Services;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Support\Facades\Concurrency;
use Illuminate\Support\Str;
use Veda\Laravel\VedaManager;

class SubagentRunner
{
    public function __construct(
        protected ToolActivityBroadcaster $broadcaster,
    ) {}

    /**
     * @param  array<int, array{type: string, input: array<string, mixed>, label?: string}>  $tasks
     * @return array{success: bool, tasks: array<int, array<string, mixed>>, error?: string}
     */
    public function run(array $tasks, ?Authenticatable $user = null, ?RequestContext $context = null): array
    {
        $maxParallel = max(1, (int) config('veda.tasks.max_parallel', 4));
        $maxPerTurn = max(1, (int) config('veda.tasks.max_per_turn', 8));

        if (count($tasks) > $maxPerTurn) {
            return [
                'success' => false,
                'error' => "Too many tasks: maximum {$maxPerTurn} per turn.",
                'tasks' => [],
            ];
        }

        $manager = app(VedaManager::class);
        $resolver = $manager->getSubagentResolver();

        if ($resolver === null) {
            return [
                'success' => false,
                'error' => 'No subagent resolver registered. Register one via Veda::subagentResolver().',
                'tasks' => [],
            ];
        }

        $tasks = array_slice(array_values($tasks), 0, $maxPerTurn);

        $states = [];
        $callbacks = [];

        foreach ($tasks as $index => $task) {
            $type = (string) ($task['type'] ?? '');
            $input = is_array($task['input'] ?? null) ? $task['input'] : [];
            $id = 'task-'.($index + 1).'-'.Str::random(6);
            $label = (string) ($task['label'] ?? $manager->labelForSubagentType($type));

            $states[$id] = [
                'id' => $id,
                'label' => $label,
                'status' => 'pending',
            ];

            $callbacks[$id] = function () use ($resolver, $type, $input, $user): array {
                $result = call_user_func($resolver, $type, $input, $user);

                return is_array($result) ? $result : ['success' => false, 'error' => 'Invalid subagent result.'];
            };
        }

        $this->broadcastStates($context, $states);

        $chunks = array_chunk($callbacks, $maxParallel, true);
        $results = [];

        foreach ($chunks as $chunk) {
            $runningStates = $states;
            foreach (array_keys($chunk) as $id) {
                $runningStates[$id]['status'] = 'running';
            }
            $this->broadcastStates($context, $runningStates);

            try {
                $chunkResults = Concurrency::run($chunk);
            } catch (\Throwable $e) {
                $chunkResults = [];
                foreach (array_keys($chunk) as $id) {
                    $chunkResults[$id] = ['success' => false, 'error' => $e->getMessage()];
                }
            }

            foreach ($chunkResults as $id => $result) {
                $results[$id] = $result;
                $states[$id]['status'] = ($result['success'] ?? false) ? 'completed' : 'failed';
                if (isset($result['summary']) && is_string($result['summary'])) {
                    $states[$id]['detail'] = mb_substr($result['summary'], 0, 200);
                }
            }

            $this->broadcastStates($context, $states);
        }

        $out = [];
        foreach ($states as $id => $state) {
            $result = $results[$id] ?? ['success' => false, 'error' => 'Task did not run.'];
            $out[] = [
                'id' => $id,
                'label' => $state['label'],
                'status' => $state['status'],
                'success' => (bool) ($result['success'] ?? false),
                'summary' => $result['summary'] ?? null,
                'data' => $result['data'] ?? null,
                'error' => $result['error'] ?? null,
            ];
        }

        return [
            'success' => true,
            'tasks' => $out,
        ];
    }

    /**
     * @param  array<string, array{id: string, label: string, status: string, detail?: string}>  $states
     */
    protected function broadcastStates(?RequestContext $context, array $states): void
    {
        $this->broadcaster->broadcastForContext($context, array_values($states));
    }
}
