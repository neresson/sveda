<?php

namespace Veda\Laravel\Prompts;

use Illuminate\Support\Str;
use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Tools\ToolNameResolver;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\Tools\VedaTool;

class PromptBuilder
{
    protected string $templatesPath;

    public function __construct(?string $templatesPath = null)
    {
        $default = dirname(__DIR__, 2).'/resources/prompts/system';
        $configured = config('veda.prompts.path');

        $this->templatesPath = $templatesPath
            ?: (is_string($configured) && $configured !== '' ? $configured : $default);
    }

    /**
     * @param  array<string, mixed>  $pageContext
     * @param  array<int, mixed>  $allowedTools
     * @param  array<int, mixed>  $firstRoundTools
     * @param  array<int, string>  $contextSections
     */
    public function buildSystemPrompt(
        array $pageContext,
        array $allowedTools,
        array $firstRoundTools = [],
        ?string $globalContextBlock = null,
        array $contextSections = [],
        ?string $conversationSummary = null,
    ): string {
        $allowedToolNames = $this->extractToolNames($allowedTools);
        $toolModeLine = $this->resolveToolModeLine($allowedTools);
        $capabilityLines = $this->buildCapabilityLines($allowedTools);
        $firstRoundDomainsLine = $this->buildFirstRoundDomainsLine($firstRoundTools);

        $sections = [];
        $sections[] = $this->renderTemplate('intro.txt');
        $sections[] = $this->renderTemplate('pull_context.txt');
        $sections[] = $this->renderTemplate('capabilities.txt', [
            'tool_mode_line' => $toolModeLine,
            'first_round_domains_line' => $firstRoundDomainsLine,
            'capabilities_block' => $capabilityLines,
        ]);

        $globalContext = trim((string) $globalContextBlock);
        if ($globalContext !== '') {
            $sections[] = $this->renderTemplate('global_context.txt', [
                'global_context_block' => $globalContext,
            ]);
        }

        $pageContextPrompt = trim($this->buildPageContextPrompt($pageContext));
        if ($pageContextPrompt !== '') {
            $sections[] = $this->renderTemplate('page_context.txt', [
                'page_context_line' => $pageContextPrompt,
            ]);
        }

        foreach ($contextSections as $section) {
            $section = trim((string) $section);
            if ($section !== '') {
                $sections[] = $section;
            }
        }

        $summary = trim((string) $conversationSummary);
        if ($summary !== '') {
            $sections[] = $this->renderTemplate('conversation_summary.txt', [
                'conversation_summary' => $summary,
            ]);
        }

        $sections[] = $this->renderTemplate('rules.txt');

        $toolSection = $this->buildToolUseInstructionsSection($allowedTools);
        if ($toolSection !== '') {
            $sections[] = $toolSection;
        }

        return trim(implode("\n\n", array_filter($sections, fn ($part) => trim((string) $part) !== '')));
    }

    /**
     * @param  array<int, mixed>  $allowedTools
     */
    public function buildToolUseInstructionsSection(array $allowedTools): string
    {
        $detector = new ToolCapabilityDetector;
        $toolUseBlock = $detector->buildToolUseInstructions($detector->detect($allowedTools));

        if (trim($toolUseBlock) === '') {
            return '';
        }

        return $this->renderTemplate('tool_use_instructions.txt', [
            'tool_use_block' => $toolUseBlock,
        ]);
    }

    /**
     * @param  array<string, mixed>  $pageContext
     * @return array<int, string>
     */
    public function buildPageDateTimeLines(array $pageContext): array
    {
        $lines = [];
        $currentDatetime = $pageContext['current_datetime'] ?? null;
        $currentDatetimeUtc = $pageContext['current_datetime_utc'] ?? null;
        $currentTimezone = $pageContext['current_timezone'] ?? null;

        if (is_string($currentDatetime) && $currentDatetime !== '') {
            $lines[] = "Current client datetime is {$currentDatetime}.";
            if (is_string($currentTimezone) && $currentTimezone !== '') {
                $lines[] = "Client timezone is {$currentTimezone}.";
            }
            if (is_string($currentDatetimeUtc) && $currentDatetimeUtc !== '') {
                $lines[] = "Current UTC datetime is {$currentDatetimeUtc}.";
            }
        }

        return $lines;
    }

    /**
     * @param  array<string, mixed>  $pageContext
     */
    public function buildPageContextPrompt(array $pageContext, bool $includeDateTime = true): string
    {
        $pageType = $pageContext['page_type'] ?? null;
        $entityId = $pageContext['entity_id'] ?? null;
        $pageTitle = $pageContext['page_title'] ?? null;
        $pageUrl = $pageContext['page_url'] ?? $pageContext['host_page_url'] ?? null;

        $prompt = '';
        if (is_string($pageType) && $pageType !== '') {
            $label = $entityId ? "{$pageType} (ID: {$entityId})" : $pageType;
            $prompt .= "User is currently viewing: {$label}. ";

            if ($entityId) {
                $prompt .= "When the user refers to 'this' or 'current' item without specifying an ID, they mean the {$pageType} with ID {$entityId}. ";
            }
        } elseif (is_string($pageTitle) && $pageTitle !== '') {
            $prompt .= "User is currently viewing: {$pageTitle}. ";
        }

        if (is_string($pageUrl) && trim($pageUrl) !== '') {
            $prompt .= "Page URL: {$pageUrl}. ";
        }

        if ($includeDateTime) {
            $dateLines = $this->buildPageDateTimeLines($pageContext);
            foreach ($dateLines as $line) {
                $prompt .= $line.' ';
            }
            if ($dateLines !== []) {
                $prompt .= "Interpret relative dates such as 'today', 'tomorrow', and 'yesterday' using the client timezone. ";
                $prompt .= 'When creating or updating records, convert and store date/time values in UTC (UTC+0). ';
            }
        }

        return $prompt;
    }

    /**
     * @param  array<int, mixed>  $tools
     * @return array<int, string>
     */
    protected function extractToolNames(array $tools): array
    {
        return array_values(array_filter(array_map(function ($tool) {
            if ($tool instanceof Tool) {
                return ToolNameResolver::resolve($tool);
            }
            $name = is_array($tool) ? ($tool['function']['name'] ?? null) : null;

            return is_string($name) && $name !== '' ? $name : null;
        }, $tools)));
    }

    /**
     * @param  array<int, mixed>  $tools
     */
    protected function resolveToolModeLine(array $tools): string
    {
        $hasWrite = false;
        $hasRead = false;

        foreach ($tools as $tool) {
            $mode = $tool instanceof VedaTool ? $tool->mode() : ToolMode::Read;
            if ($mode === ToolMode::Write) {
                $hasWrite = true;
            } else {
                $hasRead = true;
            }
        }

        if ($hasRead && $hasWrite) {
            return '- Access mode: read and write tools are available within the allowed tool list.';
        }
        if ($hasRead) {
            return '- Access mode: read-only. Do not claim or attempt create/update/delete actions.';
        }

        return '- Access mode: restricted. Use only tools explicitly listed below.';
    }

    /**
     * @param  array<int, mixed>  $tools
     * @return array<string, array{read: bool, write: bool}>
     */
    protected function buildDomainsMap(array $tools): array
    {
        $domains = [];
        foreach ($tools as $tool) {
            $domain = $tool instanceof VedaTool ? $tool->domain() : 'other';
            $mode = $tool instanceof VedaTool ? $tool->mode() : ToolMode::Read;

            if (! isset($domains[$domain])) {
                $domains[$domain] = ['read' => false, 'write' => false];
            }

            if ($mode === ToolMode::Write) {
                $domains[$domain]['write'] = true;
            } else {
                $domains[$domain]['read'] = true;
            }
        }

        ksort($domains);

        return $domains;
    }

    /**
     * @param  array<int, mixed>  $tools
     */
    protected function buildCapabilityLines(array $tools): string
    {
        $domains = $this->buildDomainsMap($tools);
        if ($domains === []) {
            return '- No business capabilities are available in this session.';
        }

        $lines = [];
        foreach ($domains as $domain => $modeSet) {
            $label = Str::of(str_replace('_', ' ', $domain))->title()->toString();
            $actions = [];
            if ($modeSet['read']) {
                $actions[] = 'view';
            }
            if ($modeSet['write']) {
                $actions[] = 'create/update';
            }
            if ($actions === []) {
                $actions[] = 'restricted';
            }
            $lines[] = '- '.$label.': '.implode(', ', $actions);
        }

        return implode("\n", $lines);
    }

    /**
     * @param  array<int, mixed>  $firstRoundTools
     */
    protected function buildFirstRoundDomainsLine(array $firstRoundTools): string
    {
        $domains = [];
        foreach ($firstRoundTools as $tool) {
            $domains[$tool instanceof VedaTool ? $tool->domain() : 'other'] = true;
        }

        if ($domains === []) {
            return '- Initial focus: general';
        }

        $labels = array_map(
            fn (string $domain): string => Str::of(str_replace('_', ' ', $domain))->title()->toString(),
            array_keys($domains)
        );

        return '- Initial focus areas: '.implode(', ', $labels);
    }

    /**
     * @param  array<string, string>  $replacements
     */
    protected function renderTemplate(string $template, array $replacements = []): string
    {
        $path = rtrim($this->templatesPath, '/').'/'.$template;
        $content = is_file($path) ? (string) file_get_contents($path) : '';
        if ($content === '') {
            return '';
        }

        foreach ($replacements as $key => $value) {
            $content = str_replace('{{'.$key.'}}', (string) $value, $content);
        }

        return trim($content);
    }
}
