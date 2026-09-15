<?php

namespace Veda\Laravel\Prompts;

use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Tools\ToolNameResolver;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Services\ToolDeferralPolicy;
use Veda\Laravel\Tools\VedaTool;

class ToolCapabilityDetector
{
    /**
     * @param  array<int, mixed>  $tools
     * @return array{has_tool_search: bool, has_write: bool, has_read: bool, has_mcp_catalog: bool}
     */
    public function detect(array $tools): array
    {
        $hasSearch = false;
        $hasWrite = false;
        $hasRead = false;
        $hasMcpCatalog = false;

        foreach ($tools as $tool) {
            $name = $this->toolName($tool);
            if ($name === null) {
                continue;
            }

            if (ToolDeferralPolicy::isSearchTool($name)) {
                $hasSearch = true;
            }

            if ($name === 'manage_mcp_catalog') {
                $hasMcpCatalog = true;
            }

            $mode = $this->toolMode($tool);
            if ($mode === ToolMode::Write) {
                $hasWrite = true;
            } else {
                $hasRead = true;
            }
        }

        return [
            'has_tool_search' => $hasSearch,
            'has_write' => $hasWrite,
            'has_read' => $hasRead,
            'has_mcp_catalog' => $hasMcpCatalog,
        ];
    }

    /**
     * @param  array{has_tool_search: bool, has_write: bool, has_read: bool, has_mcp_catalog?: bool}  $capabilities
     */
    public function buildToolUseInstructions(array $capabilities): string
    {
        $lines = [];
        $lines[] = 'The user will ask a question or request a task. Tools in this request are the only actions you may take. Tools from earlier turns may no longer be available—use only tools provided in the current request.';
        $lines[] = 'Do not make assumptions: gather context with tools until you can complete the task. Do not give up while relevant tools remain available.';
        $lines[] = 'When several independent read-only lookups are needed, issue them in one turn so they can run in parallel. After tool results, continue the task without repeating prior narration.';
        $lines[] = 'Never tell the user internal tool names; describe actions in plain language. Do not ask permission before using a tool.';

        if ($capabilities['has_tool_search'] ?? false) {
            $lines[] = 'Many tools use deferred loading: call search_agent_tools with a natural-language task description to semantically find tools, then invoke them on the next turn by exact name.';
        }

        if ($capabilities['has_write'] ?? false) {
            $lines[] = 'When mutating data, use write tools and confirm outcomes from tool results.';
        } else {
            $lines[] = 'This session is read-only: do not claim create/update/delete actions.';
        }

        if ($capabilities['has_mcp_catalog'] ?? false) {
            $lines[] = 'You cannot browse the public internet. When the user asks to use an HTTP MCP server, connect it with the MCP catalog tool, then call the returned tools in the next step. Do not tell the user to paste mcp.json unless that tool is unavailable.';
        }

        return implode("\n", $lines);
    }

    protected function toolName(mixed $tool): ?string
    {
        if ($tool instanceof Tool) {
            return ToolNameResolver::resolve($tool);
        }

        $name = is_array($tool) ? ($tool['function']['name'] ?? null) : null;

        return is_string($name) && $name !== '' ? $name : null;
    }

    protected function toolMode(mixed $tool): ToolMode
    {
        if ($tool instanceof VedaTool) {
            return $tool->mode();
        }

        return ToolMode::Read;
    }
}
