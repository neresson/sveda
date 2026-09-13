<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Contracts\JsonSchema\JsonSchema;
use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Tools\Request;
use Stringable;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Services\ToolCatalogSearchService;
use Veda\Laravel\Services\ToolDeferralPolicy;
use Veda\Laravel\VedaManager;

class SearchAgentToolsTool extends VedaTool
{
    /**
     * @param  array<string, Tool>  $deferredTools
     */
    public function __construct(
        protected array $deferredTools = [],
        ?Authenticatable $user = null,
    ) {
        parent::__construct($user);
    }

    public function name(): string
    {
        return ToolDeferralPolicy::SEARCH_TOOL_NAME;
    }

    public function description(): Stringable|string
    {
        return 'Semantic search over the agent tool catalog. Call this before using business tools that are not already available in the current request. Matching tools become available immediately.';
    }

    public function schema(JsonSchema $schema): array
    {
        return [
            'query' => $schema->string()->description('What you need to do, e.g. "create invoice" or "calendar event".')->required(),
            'domains' => $schema->array()->items($schema->string())->description('Optional domain filter, e.g. billing, calendar, users.'),
            'limit' => $schema->integer()->description('Max tools to return (default 8, max 24).'),
        ];
    }

    protected function execute(array $arguments): array|string
    {
        $query = trim((string) ($arguments['query'] ?? ''));
        if ($query === '') {
            return ['success' => false, 'error' => 'Query is required.'];
        }

        $domains = $arguments['domains'] ?? [];
        if (! is_array($domains)) {
            $domains = [];
        }
        $domains = array_values(array_filter($domains, 'is_string'));

        $limit = (int) ($arguments['limit'] ?? 8);

        $manager = app(VedaManager::class);
        $definitions = [];

        foreach ($this->deferredTools as $name => $tool) {
            $definitions[$name] = [
                'name' => $name,
                'description' => (string) $tool->description(),
                'mode' => $tool instanceof VedaTool ? $tool->mode() : ToolMode::Read,
                'domain' => $tool instanceof VedaTool ? $tool->domain() : 'other',
            ];
        }

        $result = app(ToolCatalogSearchService::class)->search($definitions, $query, $domains, $limit);

        $activated = [];
        foreach ($result['tools'] as $row) {
            $activated[] = $row['name'];
        }

        ToolDeferralPolicy::$dynamicallyActivatedTools = array_values(array_unique(array_merge(
            ToolDeferralPolicy::$dynamicallyActivatedTools,
            $activated,
        )));

        return [
            'success' => true,
            'data' => [
                'tools' => $result['tools'],
                'activated_tools' => $activated,
                'backend' => $result['backend'],
                'catalog_fingerprint' => $result['fingerprint'],
                'hint' => $activated !== []
                    ? 'These tools are now available in your next step. Call them directly by name.'
                    : 'No matching tools found. Rephrase the query or answer from your own knowledge.',
            ],
        ];
    }

    public function handle(Request $request): Stringable|string
    {
        return parent::handle($request);
    }
}
