<?php

namespace Veda\Laravel\Agent\Middleware;

use Closure;
use Laravel\Ai\Prompts\AgentPrompt;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Services\SubagentRunner;
use Veda\Laravel\VedaManager;

class RunSearchPreflight
{
    public function handle(AgentPrompt $prompt, Closure $next): mixed
    {
        if (! filter_var(config('veda.preflight.enabled', true), FILTER_VALIDATE_BOOLEAN)) {
            return $next($prompt);
        }

        $agent = $prompt->agent;
        if (! $agent instanceof VedaAgent) {
            return $next($prompt);
        }

        if ($prompt->contains('<preflight_results>')) {
            return $next($prompt);
        }

        if (trim($prompt->prompt) === '') {
            return $next($prompt);
        }

        $manager = app(VedaManager::class);
        $planner = $manager->getPreflightPlanner();

        if ($planner === null) {
            return $next($prompt);
        }

        $context = RequestContext::current();
        $pageContext = $context?->promptPageContext() ?? [];

        $taskSpecs = $planner($prompt->prompt, $pageContext, $agent->user);
        if (! is_array($taskSpecs) || $taskSpecs === []) {
            return $next($prompt);
        }

        $outcome = app(SubagentRunner::class)->run(
            array_map(fn (array $spec): array => [
                'type' => (string) ($spec['type'] ?? 'search'),
                'input' => is_array($spec['input'] ?? null) ? $spec['input'] : [],
                'label' => isset($spec['label']) ? (string) $spec['label'] : null,
            ], array_slice($taskSpecs, 0, max(1, (int) config('veda.preflight.max_parallel', 4)))),
            $agent->user,
            $context,
        );

        $brief = $this->buildBrief($outcome['tasks'] ?? []);
        if ($brief !== '') {
            $prompt = $prompt->append($brief);
        }

        return $next($prompt);
    }

    /**
     * @param  array<int, array<string, mixed>>  $tasks
     */
    protected function buildBrief(array $tasks): string
    {
        $lines = [];

        foreach ($tasks as $task) {
            $summary = trim((string) ($task['summary'] ?? ''));
            if ($summary === '') {
                continue;
            }

            $label = trim((string) ($task['label'] ?? 'task'));
            $lines[] = "### {$label}\n".$summary;
        }

        if ($lines === []) {
            return '';
        }

        return "<preflight_results>\n".implode("\n\n", $lines)."\n</preflight_results>";
    }
}
