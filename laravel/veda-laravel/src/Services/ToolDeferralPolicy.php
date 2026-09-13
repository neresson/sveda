<?php

namespace Veda\Laravel\Services;

class ToolDeferralPolicy
{
    public const SEARCH_TOOL_NAME = 'search_agent_tools';

    /**
     * @var array<int, string>
     */
    public static array $dynamicallyActivatedTools = [];

    public static bool $deferralActive = false;

    public static function isSearchTool(string $name): bool
    {
        return $name === self::SEARCH_TOOL_NAME
            || str_ends_with($name, '_'.self::SEARCH_TOOL_NAME);
    }

    public function shouldDeferTools(int $expandedToolCount, bool $eagerLoadAll = false): bool
    {
        if ($eagerLoadAll) {
            return false;
        }

        if (! (bool) config('veda.tool_defer.enabled', true)) {
            return false;
        }

        $minPool = (int) config('veda.tool_defer.min_pool', 14);

        return $expandedToolCount >= $minPool;
    }

    /**
     * @return array<int, string>
     */
    public function alwaysLoadedNames(): array
    {
        $names = config('veda.tool_defer.always_loaded', ['spawn_tasks']);

        return is_array($names) ? array_values(array_filter($names, 'is_string')) : [];
    }

    /**
     * @param  array<int, string>  $dynamicallyRequestedFromHistory
     * @return array<int, string>
     */
    public function getActiveTools(array $dynamicallyRequestedFromHistory = []): array
    {
        return array_unique(array_merge(
            $this->alwaysLoadedNames(),
            self::$dynamicallyActivatedTools,
            $dynamicallyRequestedFromHistory
        ));
    }

    /**
     * @return array<string, mixed>
     */
    public function searchToolDefinition(): array
    {
        return [
            'type' => 'function',
            'function' => [
                'name' => self::SEARCH_TOOL_NAME,
                'description' => 'Semantic search over the agent tool catalog (embeddings with keyword fallback). Returns matching tool names and short descriptions. Call this before using business tools that are not already available in the current request. Matching tools become available immediately.',
                'parameters' => [
                    'type' => 'object',
                    'properties' => [
                        'query' => [
                            'type' => 'string',
                            'description' => 'What you need to do, e.g. "create invoice" or "calendar event".',
                        ],
                        'domains' => [
                            'type' => 'array',
                            'items' => ['type' => 'string'],
                            'description' => 'Optional domain filter, e.g. billing, calendar, users.',
                        ],
                        'limit' => [
                            'type' => 'integer',
                            'description' => 'Max tools to return (default 8, max 24).',
                        ],
                    ],
                    'required' => ['query'],
                ],
            ],
        ];
    }
}
