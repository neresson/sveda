<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Http\Request;
use Veda\Laravel\Models\VedaGeneration;
use Veda\Laravel\Services\VedaAdminUsage;
use Veda\Laravel\Tests\TestCase;

class AdminUsageTest extends TestCase
{
    public function test_snapshot_is_empty_when_there_is_no_traffic(): void
    {
        $snapshot = app(VedaAdminUsage::class)->snapshot(Request::create('/veda/admin/usage', 'GET'));

        $this->assertSame([], $snapshot['by_model']);
        $this->assertSame([], $snapshot['requests']['data']);
        $this->assertSame(0, $snapshot['requests']['total']);
        $this->assertSame(1, $snapshot['requests']['current_page']);
    }

    public function test_snapshot_groups_tokens_by_model_and_lists_requests_without_prompts(): void
    {
        $this->createGeneration([
            'model' => 'deepseek-v4-flash',
            'prompt' => 'secret-one',
            'tokens_used' => 100,
        ]);
        $this->createGeneration([
            'model' => 'deepseek-v4-flash',
            'prompt' => 'secret-two',
            'tokens_used' => 50,
        ]);
        $this->createGeneration([
            'model' => 'gpt-mini',
            'prompt' => 'secret-three',
            'tokens_used' => 20,
        ]);

        $snapshot = app(VedaAdminUsage::class)->snapshot(Request::create('/veda/admin/usage', 'GET'));

        $this->assertSame('deepseek-v4-flash', $snapshot['by_model'][0]['model']);
        $this->assertSame(2, $snapshot['by_model'][0]['requests']);
        $this->assertSame(150, $snapshot['by_model'][0]['tokens_used']);
        $this->assertSame('gpt-mini', $snapshot['by_model'][1]['model']);
        $this->assertSame(1, $snapshot['by_model'][1]['requests']);
        $this->assertSame(20, $snapshot['by_model'][1]['tokens_used']);

        $this->assertCount(3, $snapshot['requests']['data']);
        $this->assertSame(3, $snapshot['requests']['total']);
        $this->assertSame('gpt-mini', $snapshot['requests']['data'][0]['model']);
        $this->assertSame(20, $snapshot['requests']['data'][0]['tokens_used']);
        $this->assertArrayNotHasKey('prompt', $snapshot['requests']['data'][0]);
        $this->assertArrayNotHasKey('generated_content', $snapshot['requests']['data'][0]);
    }

    public function test_snapshot_paginates_requests(): void
    {
        for ($index = 1; $index <= 26; $index++) {
            $this->createGeneration([
                'model' => 'deepseek-v4-flash',
                'tokens_used' => $index,
            ]);
        }

        $pageTwo = app(VedaAdminUsage::class)->snapshot(Request::create('/veda/admin/usage?p=2', 'GET'));

        $this->assertSame(2, $pageTwo['requests']['current_page']);
        $this->assertSame(2, $pageTwo['requests']['last_page']);
        $this->assertCount(1, $pageTwo['requests']['data']);
        $this->assertSame(26, $pageTwo['requests']['total']);
        $this->assertNotNull($pageTwo['requests']['prev_page_url']);
        $this->assertNull($pageTwo['requests']['next_page_url']);
    }

    /**
     * @param  array<string, mixed>  $attributes
     */
    private function createGeneration(array $attributes): VedaGeneration
    {
        return VedaGeneration::query()->create(array_merge([
            'generation_type' => 'chat',
            'prompt' => 'hello',
            'status' => VedaGeneration::STATUS_COMPLETED,
            'tokens_used' => 10,
        ], $attributes));
    }
};
