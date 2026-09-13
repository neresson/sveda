<?php

namespace Veda\Laravel\Gateway\Concerns;

use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\Concerns\InvokesTools;
use Laravel\Ai\Tools\ToolNameResolver;

trait InvokesToolsByName
{
    use InvokesTools {
        findTool as protected findToolByExactName;
    }
    use MapsActiveTools;

    protected function findTool(string $name, array $tools): ?Tool
    {
        $tool = $this->findToolByExactName($name, $tools);
        if ($tool !== null) {
            return $tool;
        }

        $normalized = $this->normalizeToolName($name);
        if ($normalized === $name) {
            return null;
        }

        foreach ($tools as $candidate) {
            if (! $candidate instanceof Tool) {
                continue;
            }

            if ($this->normalizeToolName(ToolNameResolver::resolve($candidate)) === $normalized) {
                return $candidate;
            }
        }

        return null;
    }
}
