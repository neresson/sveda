<?php

namespace Veda\Laravel\Services;

use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\ToolResult;

final class RequestContext
{
    private static ?self $instance = null;

    /**
     * @var array<string, array<string, mixed>>
     */
    private array $toolResultCache = [];

    /**
     * @param  array<string, mixed>  $pageContext
     * @param  array<int, array{name: string, description?: string, parameters?: array<string, mixed>}>  $clientTools
     * @param  array<int, ToolCall>|null  $pendingToolCalls
     * @param  array<int, ToolResult>|null  $pendingToolResults
     */
    public function __construct(
        public readonly array $pageContext,
        public readonly bool $isEmbedMode,
        public readonly bool $isEmbedGuest = false,
        public readonly int|string|null $userId = null,
        public readonly ?string $chatId = null,
        public readonly ?string $runId = null,
        public readonly array $clientTools = [],
        public readonly ?array $pendingToolCalls = null,
        public readonly ?array $pendingToolResults = null,
        public readonly ?bool $thinkingEnabled = null,
    ) {}

    /**
     * @param  array<string, mixed>  $pageContext
     * @param  array<int, array<string, mixed>>  $clientTools
     * @param  array<int, ToolCall>|null  $pendingToolCalls
     * @param  array<int, ToolResult>|null  $pendingToolResults
     */
    public static function bind(
        array $pageContext,
        bool $isEmbedMode,
        int|string|null $userId = null,
        ?string $chatId = null,
        ?string $runId = null,
        bool $isEmbedGuest = false,
        array $clientTools = [],
        ?array $pendingToolCalls = null,
        ?array $pendingToolResults = null,
        ?bool $thinkingEnabled = null,
    ): self {
        self::$instance = new self(
            $pageContext,
            $isEmbedMode,
            $isEmbedGuest,
            $userId,
            $chatId,
            $runId,
            $clientTools,
            $pendingToolCalls,
            $pendingToolResults,
            $thinkingEnabled,
        );

        return self::$instance;
    }

    public static function current(): ?self
    {
        return self::$instance;
    }

    public static function forget(): void
    {
        self::$instance = null;
    }

    public function hasHostPage(): bool
    {
        $url = $this->pageContext['host_page_url'] ?? null;

        return is_string($url) && trim($url) !== '';
    }

    /**
     * @return array<string, mixed>
     */
    public function promptPageContext(): array
    {
        $context = $this->pageContext;
        unset($context['host_interactive_outline']);

        return $context;
    }

    /**
     * @return array<string, mixed>
     */
    public function fullPageContext(): array
    {
        return $this->pageContext;
    }

    /**
     * @return array<int, array<string, mixed>>
     */
    public function hostOutline(): array
    {
        $outline = $this->pageContext['host_interactive_outline'] ?? null;

        return is_array($outline) ? $outline : [];
    }

    /**
     * @return array<int, string>
     */
    public function clientToolNames(): array
    {
        $names = [];
        foreach ($this->clientTools as $tool) {
            $name = $tool['name'] ?? null;
            if (is_string($name) && $name !== '') {
                $names[] = $name;
            }
        }

        return $names;
    }

    public function hasClientTool(string $name): bool
    {
        return in_array($name, $this->clientToolNames(), true);
    }

    public function hasPendingToolContinuation(): bool
    {
        return is_array($this->pendingToolResults) && $this->pendingToolResults !== [];
    }

    public function toolCacheKey(string $toolName, array $arguments): string
    {
        $encoded = json_encode($arguments, JSON_UNESCAPED_UNICODE | JSON_INVALID_UTF8_SUBSTITUTE);

        return $toolName.':'.hash('sha256', is_string($encoded) ? $encoded : '');
    }

    /**
     * @return array<string, mixed>|null
     */
    public function recallToolResult(string $cacheKey): ?array
    {
        $cached = $this->toolResultCache[$cacheKey] ?? null;

        return is_array($cached) ? $cached : null;
    }

    /**
     * @param  array<string, mixed>  $result
     */
    public function rememberToolResult(string $cacheKey, array $result): void
    {
        $this->toolResultCache[$cacheKey] = $result;
    }
}
