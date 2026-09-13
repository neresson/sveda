<?php

namespace Veda\Laravel;

use Illuminate\Contracts\Auth\Authenticatable;
use Laravel\Ai\Contracts\Agent;
use Laravel\Ai\Contracts\Tool;
use Veda\Laravel\Contracts\TokenPolicy;
use Veda\Laravel\Contracts\ToolPermissionHook;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Support\AllowAllToolsPermissionHook;
use Veda\Laravel\Support\NullTokenPolicy;
use Veda\Laravel\Tools\ClosureTool;
use Veda\Laravel\Tools\VedaTool;

class VedaManager
{
    /**
     * @var array<string, array{name: string, description: string, schema: array<string, mixed>|callable, handler: callable, mode: ToolMode, domain: string}>
     */
    protected array $toolDefinitions = [];

    /**
     * @var array<int, class-string<VedaTool>>
     */
    protected array $toolClasses = [];

    /**
     * @var array<string, string>
     */
    protected array $toolAliases = [];

    /**
     * @var array<int, callable>
     */
    protected array $contextProviders = [];

    protected ?TokenPolicy $tokenPolicy = null;

    protected ?ToolPermissionHook $permissionHook = null;

    /**
     * @var callable|null
     */
    protected $toolResultEnricher = null;

    /**
     * @var callable|null
     */
    protected $subagentResolver = null;

    /**
     * @var callable|null
     */
    protected $preflightPlanner = null;

    /**
     * @var callable|null
     */
    protected $documentExtractor = null;

    /**
     * @var callable|null
     */
    protected $responseGuard = null;

    /**
     * @var callable|null
     */
    protected $globalContextProvider = null;

    /**
     * @var array<string, string>
     */
    protected array $subagentLabels = [];

    /**
     * @var callable|null
     */
    protected $toolNameNormalizer = null;

    /**
     * @param  array<string, mixed>|callable  $schema
     */
    public function tool(
        string $name,
        string $description,
        array|callable $schema,
        callable $handler,
        ToolMode $mode = ToolMode::Read,
        string $domain = 'other',
    ): self {
        $this->toolDefinitions[$name] = [
            'name' => $name,
            'description' => $description,
            'schema' => $schema,
            'handler' => $handler,
            'mode' => $mode,
            'domain' => $domain,
        ];

        return $this;
    }

    /**
     * @param  class-string<VedaTool>  $class
     */
    public function toolClass(string $class): self
    {
        if (! in_array($class, $this->toolClasses, true)) {
            $this->toolClasses[] = $class;
        }

        return $this;
    }

    public function toolAlias(string $alias, string $canonicalName): self
    {
        $this->toolAliases[$alias] = $canonicalName;

        return $this;
    }

    public function contextProvider(callable $provider): self
    {
        $this->contextProviders[] = $provider;

        return $this;
    }

    public function globalContextProvider(callable $provider): self
    {
        $this->globalContextProvider = $provider;

        return $this;
    }

    /**
     * @param  array<string, mixed>  $pageContext
     */
    public function resolveGlobalContextBlock(?Authenticatable $user, array $pageContext = [], ?RequestContext $context = null): ?string
    {
        if ($this->globalContextProvider === null) {
            return null;
        }

        $block = call_user_func($this->globalContextProvider, $user, $pageContext, $context);

        if (! is_string($block) || trim($block) === '') {
            return null;
        }

        return trim($block);
    }

    public function tokenPolicy(TokenPolicy $policy): self
    {
        $this->tokenPolicy = $policy;

        return $this;
    }

    public function getTokenPolicy(): TokenPolicy
    {
        return $this->tokenPolicy ??= new NullTokenPolicy;
    }

    public function permissionHook(ToolPermissionHook $hook): self
    {
        $this->permissionHook = $hook;

        return $this;
    }

    public function getPermissionHook(): ToolPermissionHook
    {
        return $this->permissionHook ??= new AllowAllToolsPermissionHook;
    }

    public function toolResultEnricher(callable $enricher): self
    {
        $this->toolResultEnricher = $enricher;

        return $this;
    }

    /**
     * @param  array<string, mixed>  $arguments
     * @param  array<string, mixed>  $result
     * @return array<string, mixed>
     */
    public function enrichToolResult(string $toolName, array $arguments, array $result): array
    {
        if ($this->toolResultEnricher === null) {
            return $result;
        }

        $enriched = call_user_func($this->toolResultEnricher, $toolName, $arguments, $result);

        return is_array($enriched) ? $enriched : $result;
    }

    public function subagentResolver(callable $resolver): self
    {
        $this->subagentResolver = $resolver;

        return $this;
    }

    public function getSubagentResolver(): ?callable
    {
        return $this->subagentResolver;
    }

    public function resolveSubagent(string $type, ?Authenticatable $user): ?Agent
    {
        if ($this->subagentResolver === null) {
            return null;
        }

        $agent = call_user_func($this->subagentResolver, $type, $user);

        return $agent instanceof Agent ? $agent : null;
    }

    public function subagentLabel(string $type, string $label): self
    {
        $this->subagentLabels[$type] = $label;

        return $this;
    }

    public function labelForSubagentType(string $type): string
    {
        return $this->subagentLabels[$type] ?? $type;
    }

    public function preflightPlanner(callable $planner): self
    {
        $this->preflightPlanner = $planner;

        return $this;
    }

    public function getPreflightPlanner(): ?callable
    {
        return $this->preflightPlanner;
    }

    /**
     * @param  array<string, mixed>  $pageContext
     * @return array<int, array{type: string, description: string}>
     */
    public function planPreflight(string $prompt, array $pageContext): array
    {
        if ($this->preflightPlanner === null) {
            return [];
        }

        $plan = call_user_func($this->preflightPlanner, $prompt, $pageContext);

        return is_array($plan) ? $plan : [];
    }

    public function documentExtractor(callable $extractor): self
    {
        $this->documentExtractor = $extractor;

        return $this;
    }

    public function getDocumentExtractor(): ?callable
    {
        return $this->documentExtractor;
    }

    public function extractDocumentText(string $path, string $mimeType): ?string
    {
        if ($this->documentExtractor === null) {
            return null;
        }

        $text = call_user_func($this->documentExtractor, $path, $mimeType);

        return is_string($text) ? $text : null;
    }

    public function responseGuard(callable $guard): self
    {
        $this->responseGuard = $guard;

        return $this;
    }

    /**
     * @param  array<string, mixed>  $context
     */
    public function applyResponseGuard(string $text, array $context = []): string
    {
        if ($this->responseGuard === null) {
            return $text;
        }

        $guarded = call_user_func($this->responseGuard, $text, $context);

        return is_string($guarded) ? $guarded : $text;
    }

    /**
     * @return array<int, Tool>
     */
    public function toolInstances(?Authenticatable $user = null): array
    {
        $instances = [];

        foreach ($this->toolClasses as $class) {
            $instances[] = new $class($user);
        }

        foreach ($this->toolDefinitions as $definition) {
            $instances[] = new ClosureTool(
                $definition['name'],
                $definition['description'],
                $definition['schema'],
                $definition['handler'],
                $definition['mode'],
                $definition['domain'],
                $user,
            );
        }

        return $instances;
    }

    /**
     * @return array<string, array{name: string, description: string, schema: array<string, mixed>|callable, handler: callable, mode: ToolMode, domain: string}>
     */
    public function toolDefinitions(): array
    {
        return $this->toolDefinitions;
    }

    /**
     * @return array<int, class-string<VedaTool>>
     */
    public function toolClasses(): array
    {
        return $this->toolClasses;
    }

    /**
     * @return array<string, string>
     */
    public function toolAliases(): array
    {
        return $this->toolAliases;
    }

    public function toolNameNormalizer(callable $normalizer): self
    {
        $this->toolNameNormalizer = $normalizer;

        return $this;
    }

    public function normalizeToolName(string $toolName): string
    {
        $normalized = $this->toolAliases[$toolName] ?? $toolName;

        if ($this->toolNameNormalizer !== null) {
            $custom = call_user_func($this->toolNameNormalizer, $normalized);

            return is_string($custom) && $custom !== '' ? $custom : $normalized;
        }

        return $normalized;
    }

    /**
     * @return array<int, callable>
     */
    public function contextProviders(): array
    {
        return $this->contextProviders;
    }

    public function flush(): void
    {
        $this->toolDefinitions = [];
        $this->toolClasses = [];
        $this->toolAliases = [];
        $this->contextProviders = [];
        $this->tokenPolicy = null;
        $this->permissionHook = null;
        $this->toolResultEnricher = null;
        $this->toolNameNormalizer = null;
        $this->subagentResolver = null;
        $this->preflightPlanner = null;
        $this->documentExtractor = null;
        $this->responseGuard = null;
        $this->globalContextProvider = null;
        $this->subagentLabels = [];
    }
}
