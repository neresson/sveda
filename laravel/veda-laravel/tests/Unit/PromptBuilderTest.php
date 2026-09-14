<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Prompts\PromptBuilder;
use Veda\Laravel\Tests\Fixtures\DummyReadTool;
use Veda\Laravel\Tests\Fixtures\DummyWriteTool;
use Veda\Laravel\Tests\TestCase;

class PromptBuilderTest extends TestCase
{
    public function test_builds_prompt_with_core_sections(): void
    {
        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([], []);

        $this->assertStringContainsString('Veda', $prompt);
        $this->assertNotEmpty($prompt);
    }

    public function test_includes_page_context_section(): void
    {
        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([
            'page_type' => 'product',
            'entity_id' => 42,
            'page_title' => 'Sample product',
            'page_url' => 'https://example.com/products/42',
        ], []);

        $this->assertStringContainsString('product', $prompt);
        $this->assertStringContainsString('ID: 42', $prompt);
        $this->assertStringContainsString('https://example.com/products/42', $prompt);
    }

    public function test_includes_conversation_summary_section(): void
    {
        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([], [], [], null, [], 'User asked about invoices.');

        $this->assertStringContainsString('User asked about invoices.', $prompt);
    }

    public function test_includes_host_context_sections(): void
    {
        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([], [], [], null, ['HOST SECTION: billing rules here.']);

        $this->assertStringContainsString('HOST SECTION: billing rules here.', $prompt);
    }

    public function test_includes_global_context_block(): void
    {
        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([], [], [], 'Global knowledge base excerpt.');

        $this->assertStringContainsString('Global knowledge base excerpt.', $prompt);
    }

    public function test_tool_instructions_reflect_read_and_write_tools(): void
    {
        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([], [new DummyReadTool, new DummyWriteTool]);

        $this->assertStringContainsString('read and write', $prompt);
        $this->assertStringContainsString('Testing', $prompt);
    }

    public function test_read_only_tools_produce_read_only_instructions(): void
    {
        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([], [new DummyReadTool]);

        $this->assertStringContainsString('read-only', $prompt);
    }

    public function test_includes_instance_system_prompt(): void
    {
        config()->set('veda.system_prompt', 'Always answer in Russian.');

        $builder = new PromptBuilder;

        $prompt = $builder->buildSystemPrompt([], []);

        $this->assertStringContainsString('Always answer in Russian.', $prompt);
    }
}
