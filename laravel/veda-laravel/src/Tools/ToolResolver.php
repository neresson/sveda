<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Support\Str;
use Laravel\Ai\Contracts\Tool;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Services\ToolDeferralPolicy;
use Veda\Laravel\VedaManager;

class ToolResolver
{
    public function __construct(
        protected VedaManager $manager,
        protected ToolDeferralPolicy $deferralPolicy,
    ) {}

    /**
     * @return array{tools: array<int, Tool>, deferred: bool, expandedCount: int, activeTools: array<int, string>}
     */
    public function resolve(
        ?Authenticatable $user = null,
        ?RequestContext $context = null,
        bool $eagerLoadAll = false,
    ): array {
        $instances = $this->manager->toolInstances($user);

        $instances = array_filter(
            $instances,
            fn (Tool $tool): bool => $this->manager->getPermissionHook()->allows(
                $user,
                $this->toolName($tool),
                $this->toolMode($tool),
            )
        );

        $isEmbed = $context?->isEmbedMode ?? false;
        $writeToolsEnabled = $this->embedWriteToolsEnabled($user);

        if ($isEmbed && ! $writeToolsEnabled) {
            $instances = array_filter(
                $instances,
                fn (Tool $tool): bool => $this->toolMode($tool) === ToolMode::Read
            );
        }

        $expandedCount = count($instances);
        $deferred = $this->deferralPolicy->shouldDeferTools($expandedCount, $eagerLoadAll);

        if (! $eagerLoadAll) {
            ToolDeferralPolicy::$deferralActive = $deferred;
        }

        if (! $deferred) {
            return [
                'tools' => array_values($instances),
                'deferred' => false,
                'expandedCount' => $expandedCount,
                'activeTools' => [],
            ];
        }

        $activeNames = $this->deferralPolicy->getActiveTools();
        $active = [];
        $deferredTools = [];

        foreach ($instances as $tool) {
            $name = $this->toolName($tool);
            if (in_array($name, $activeNames, true)) {
                $active[] = $tool;
            } else {
                $deferredTools[$name] = $tool;
            }
        }

        $active[] = new SearchAgentToolsTool($deferredTools, $user);

        return [
            'tools' => $active,
            'deferred' => true,
            'expandedCount' => $expandedCount,
            'activeTools' => $activeNames,
        ];
    }

    /**
     * @return array<string, Tool>
     */
    public function instancesByName(?Authenticatable $user = null): array
    {
        $byName = [];

        foreach ($this->manager->toolInstances($user) as $tool) {
            $byName[$this->toolName($tool)] = $tool;
        }

        return $byName;
    }

    public function toolName(Tool $tool): string
    {
        if ($tool instanceof VedaTool) {
            return $tool->name();
        }

        $class = $tool::class;

        return Str::snake(class_basename($class));
    }

    public function toolMode(Tool $tool): ToolMode
    {
        if ($tool instanceof VedaTool) {
            return $tool->mode();
        }

        return ToolMode::Read;
    }

    protected function embedWriteToolsEnabled(?Authenticatable $user): bool
    {
        return (bool) config('veda.embed.write_tools_enabled', false);
    }
}
