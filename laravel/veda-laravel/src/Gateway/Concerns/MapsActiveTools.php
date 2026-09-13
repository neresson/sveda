<?php

namespace Veda\Laravel\Gateway\Concerns;

use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Tools\ToolNameResolver;
use Veda\Laravel\Services\ToolDeferralPolicy;
use Veda\Laravel\VedaManager;

trait MapsActiveTools
{
    /**
     * @param  array<int, string>  $activeTools
     */
    public function isActiveTool(string $toolName, array $activeTools): bool
    {
        if (ToolDeferralPolicy::isSearchTool($toolName)) {
            return true;
        }

        $normalized = $this->normalizeToolName($toolName);

        foreach ($activeTools as $activeTool) {
            if (! is_string($activeTool) || $activeTool === '') {
                continue;
            }

            if ($this->normalizeToolName($activeTool) === $normalized) {
                return true;
            }
        }

        return false;
    }

    /**
     * @param  array<int, string>  $activeTools
     */
    protected function shouldMapTool(Tool $tool, array $activeTools): bool
    {
        if (! (bool) config('veda.tool_defer.enabled', true)) {
            return true;
        }

        if (! ToolDeferralPolicy::$deferralActive) {
            return true;
        }

        return $this->isActiveTool(ToolNameResolver::resolve($tool), $activeTools);
    }

    protected function normalizeToolName(string $name): string
    {
        return strtolower(trim(app(VedaManager::class)->normalizeToolName($name)));
    }

    /**
     * @return array<int, string>
     */
    protected function activeToolNames(): array
    {
        return app(ToolDeferralPolicy::class)->getActiveTools();
    }
}
