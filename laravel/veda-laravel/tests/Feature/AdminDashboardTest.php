<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Support\Carbon;
use Laravel\Ai\Responses\Data\Usage;
use Veda\Laravel\Models\VedaChatHistory;
use Veda\Laravel\Models\VedaGeneration;
use Veda\Laravel\Services\VedaAdminDashboard;
use Veda\Laravel\Tests\TestCase;

class AdminDashboardTest extends TestCase
{
    protected function tearDown(): void
    {
        Carbon::setTestNow();

        parent::tearDown();
    }

    public function test_mark_as_completed_stores_prompt_and_completion_tokens(): void
    {
        $generation = VedaGeneration::query()->create([
            'generation_type' => 'chat',
            'prompt' => 'hello',
            'status' => VedaGeneration::STATUS_PENDING,
        ]);

        $generation->markAsCompleted(['explanation' => 'ok'], 30, 10, 20);
        $generation->refresh();

        $this->assertSame(VedaGeneration::STATUS_COMPLETED, $generation->status);
        $this->assertSame(30, $generation->tokens_used);
        $this->assertSame(10, $generation->prompt_tokens);
        $this->assertSame(20, $generation->completion_tokens);
    }

    public function test_snapshot_is_empty_when_there_is_no_traffic(): void
    {
        Carbon::setTestNow(Carbon::parse('2026-09-14 12:00:00'));

        $snapshot = app(VedaAdminDashboard::class)->snapshot();

        $this->assertSame(14, $snapshot['period_days']);
        $this->assertSame('2026-09-01', $snapshot['from']);
        $this->assertSame('2026-09-14', $snapshot['to']);
        $this->assertSame(0, $snapshot['requests']);
        $this->assertSame(0, $snapshot['tokens_used']);
        $this->assertCount(14, $snapshot['series']);
        $this->assertSame('2026-09-01', $snapshot['series'][0]['date']);
        $this->assertSame('2026-09-14', $snapshot['series'][13]['date']);
    }

    public function test_snapshot_aggregates_requests_and_tokens_for_the_period(): void
    {
        Carbon::setTestNow(Carbon::parse('2026-09-14 12:00:00'));

        $this->createGeneration([
            'status' => VedaGeneration::STATUS_COMPLETED,
            'user_id' => 1,
            'prompt_tokens' => 10,
            'completion_tokens' => 20,
            'tokens_used' => 30,
            'created_at' => Carbon::parse('2026-09-14 09:00:00'),
        ]);
        $this->createGeneration([
            'status' => VedaGeneration::STATUS_COMPLETED,
            'user_id' => 2,
            'prompt_tokens' => 5,
            'completion_tokens' => 15,
            'tokens_used' => 20,
            'created_at' => Carbon::parse('2026-09-13 18:00:00'),
        ]);
        $this->createGeneration([
            'status' => VedaGeneration::STATUS_FAILED,
            'tokens_used' => 0,
            'created_at' => Carbon::parse('2026-09-13 10:00:00'),
        ]);
        $this->createGeneration([
            'status' => VedaGeneration::STATUS_PENDING,
            'created_at' => Carbon::parse('2026-09-14 11:00:00'),
        ]);
        $this->createGeneration([
            'status' => VedaGeneration::STATUS_COMPLETED,
            'prompt_tokens' => 100,
            'completion_tokens' => 200,
            'tokens_used' => 300,
            'created_at' => Carbon::parse('2026-08-01 12:00:00'),
        ]);

        VedaChatHistory::query()->create([
            'chat_id' => 'chat-1',
            'visitor_id' => '',
            'messages' => [],
            'created_at' => Carbon::parse('2026-09-13 12:00:00'),
            'updated_at' => Carbon::parse('2026-09-13 12:00:00'),
        ]);

        $snapshot = app(VedaAdminDashboard::class)->snapshot();

        $this->assertSame(4, $snapshot['requests']);
        $this->assertSame(2, $snapshot['completed']);
        $this->assertSame(1, $snapshot['failed']);
        $this->assertSame(1, $snapshot['pending']);
        $this->assertSame(15, $snapshot['prompt_tokens']);
        $this->assertSame(35, $snapshot['completion_tokens']);
        $this->assertSame(50, $snapshot['tokens_used']);
        $this->assertSame(2, $snapshot['users']);
        $this->assertSame(1, $snapshot['conversations']);
        $this->assertSame(25, $snapshot['avg_tokens']);
        $this->assertSame(50, $snapshot['success_rate']);
        $this->assertSame(0, $snapshot['unsplit_tokens']);

        $byDate = collect($snapshot['series'])->keyBy('date');
        $this->assertSame(2, $byDate['2026-09-14']['requests']);
        $this->assertSame(2, $byDate['2026-09-13']['requests']);
        $this->assertSame(30, $byDate['2026-09-14']['tokens_used']);
        $this->assertSame(20, $byDate['2026-09-13']['tokens_used']);
        $this->assertSame(0, $byDate['2026-09-01']['requests']);
    }

    public function test_tokens_from_usage_includes_cache_read_input(): void
    {
        $tokens = VedaGeneration::tokensFromUsage(new Usage(
            promptTokens: 225,
            completionTokens: 1,
            cacheReadInputTokens: 17362,
        ));

        $this->assertSame(17587, $tokens['prompt_tokens']);
        $this->assertSame(1, $tokens['completion_tokens']);
        $this->assertSame(17588, $tokens['tokens_used']);
    }

    public function test_snapshot_reports_unsplit_tokens_from_legacy_rows(): void
    {
        Carbon::setTestNow(Carbon::parse('2026-09-14 12:00:00'));

        $this->createGeneration([
            'status' => VedaGeneration::STATUS_COMPLETED,
            'tokens_used' => 100,
            'created_at' => Carbon::parse('2026-09-13 10:00:00'),
        ]);
        $this->createGeneration([
            'status' => VedaGeneration::STATUS_COMPLETED,
            'prompt_tokens' => 10,
            'completion_tokens' => 5,
            'tokens_used' => 15,
            'created_at' => Carbon::parse('2026-09-14 09:00:00'),
        ]);

        $snapshot = app(VedaAdminDashboard::class)->snapshot();

        $this->assertSame(115, $snapshot['tokens_used']);
        $this->assertSame(10, $snapshot['prompt_tokens']);
        $this->assertSame(5, $snapshot['completion_tokens']);
        $this->assertSame(100, $snapshot['unsplit_tokens']);

        $byDate = collect($snapshot['series'])->keyBy('date');
        $this->assertSame(100, $byDate['2026-09-13']['unsplit_tokens']);
        $this->assertSame(0, $byDate['2026-09-14']['unsplit_tokens']);
    }

    /**
     * @param  array<string, mixed>  $attributes
     */
    private function createGeneration(array $attributes): VedaGeneration
    {
        $createdAt = $attributes['created_at'] ?? now();
        unset($attributes['created_at']);

        $generation = VedaGeneration::query()->create(array_merge([
            'generation_type' => 'chat',
            'prompt' => 'hello',
            'status' => VedaGeneration::STATUS_COMPLETED,
        ], $attributes));

        $generation->created_at = $createdAt;
        $generation->updated_at = $createdAt;
        $generation->save();

        return $generation;
    }
}
