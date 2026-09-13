<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Laravel\Mcp\Client\Primitives\Tool as McpClientTool;
use Stringable;
use Veda\Laravel\Enums\ToolMode;

class HostMcpVedaTool extends VedaTool
{
    public function __construct(protected McpClientTool $mcpTool)
    {
        parent::__construct(null);
    }

    public function name(): string
    {
        return $this->mcpTool->name;
    }

    public function description(): Stringable|string
    {
        $description = $this->mcpTool->description ?? $this->mcpTool->title ?? $this->mcpTool->name;

        return is_string($description) && $description !== '' ? $description : $this->mcpTool->name;
    }

    public function mode(): ToolMode
    {
        $mode = $this->mcpTool->meta['mode'] ?? null;
        if (! is_string($mode)) {
            $readOnly = $this->mcpTool->annotations['readOnlyHint'] ?? null;
            $mode = $readOnly === false ? ToolMode::Write->value : ToolMode::Read->value;
        }

        return $mode === ToolMode::Write->value ? ToolMode::Write : ToolMode::Read;
    }

    public function domain(): string
    {
        $domain = $this->mcpTool->meta['domain'] ?? null;

        return is_string($domain) && $domain !== '' ? $domain : 'other';
    }

    public function schema(JsonSchema $schema): array
    {
        return (new ArraySchemaConverter)->convert($schema, $this->mcpTool->inputSchema);
    }

    protected function execute(array $arguments): array|string
    {
        $result = $this->mcpTool->call($arguments);

        if ($result->isError) {
            $text = $result->text();

            return [
                'success' => false,
                'error' => $text !== '' ? $text : 'MCP tool error.',
            ];
        }

        if (is_array($result->structuredContent) && $result->structuredContent !== []) {
            return $result->structuredContent;
        }

        $text = $result->text();
        if ($text === '') {
            return ['success' => true, 'data' => null];
        }

        return $text;
    }
}
