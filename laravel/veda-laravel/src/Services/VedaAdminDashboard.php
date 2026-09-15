<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\Schema;
use Veda\Laravel\Models\VedaChatHistory;
use Veda\Laravel\Models\VedaGeneration;

class VedaAdminDashboard
{
    public const DAYS = 14;

    /**
     * @return array{
     *     period_days: int,
     *     from: string,
     *     to: string,
     *     requests: int,
     *     completed: int,
     *     failed: int,
     *     pending: int,
     *     prompt_tokens: int,
     *     completion_tokens: int,
     *     tokens_used: int,
     *     unsplit_tokens: int,
     *     users: int,
     *     conversations: int,
     *     avg_tokens: int,
     *     success_rate: int,
     *     series: list<array{date: string, requests: int, prompt_tokens: int, completion_tokens: int, tokens_used: int, unsplit_tokens: int}>
     * }
     */
    public function snapshot(?Carbon $now = null): array
    {
        $now = ($now ?? now())->copy();
        $from = $now->copy()->startOfDay()->subDays(self::DAYS - 1);
        $to = $now->copy()->endOfDay();

        if (! Schema::hasTable((new VedaGeneration)->getTable())) {
            return $this->emptySnapshot($from, $to);
        }

        $totals = VedaGeneration::query()
            ->whereBetween('created_at', [$from, $to])
            ->selectRaw('count(*) as requests')
            ->selectRaw("coalesce(sum(case when status = 'completed' then 1 else 0 end), 0) as completed")
            ->selectRaw("coalesce(sum(case when status = 'failed' then 1 else 0 end), 0) as failed")
            ->selectRaw("coalesce(sum(case when status = 'pending' then 1 else 0 end), 0) as pending")
            ->selectRaw('coalesce(sum(prompt_tokens), 0) as prompt_tokens')
            ->selectRaw('coalesce(sum(completion_tokens), 0) as completion_tokens')
            ->selectRaw('coalesce(sum(tokens_used), 0) as tokens_used')
            ->selectRaw('count(distinct user_id) as users')
            ->first();

        $requests = $this->int($totals?->requests);
        $completed = $this->int($totals?->completed);
        $promptTokens = $this->int($totals?->prompt_tokens);
        $completionTokens = $this->int($totals?->completion_tokens);
        $tokensUsed = $this->int($totals?->tokens_used);

        return [
            'period_days' => self::DAYS,
            'from' => $from->toDateString(),
            'to' => $to->toDateString(),
            'requests' => $requests,
            'completed' => $completed,
            'failed' => $this->int($totals?->failed),
            'pending' => $this->int($totals?->pending),
            'prompt_tokens' => $promptTokens,
            'completion_tokens' => $completionTokens,
            'tokens_used' => $tokensUsed,
            'unsplit_tokens' => $this->unsplit($tokensUsed, $promptTokens, $completionTokens),
            'users' => $this->int($totals?->users),
            'conversations' => $this->conversationCount($from, $to),
            'avg_tokens' => $completed > 0 ? (int) round($tokensUsed / $completed) : 0,
            'success_rate' => $requests > 0 ? (int) round(($completed / $requests) * 100) : 0,
            'series' => $this->series($from, $to),
        ];
    }

    /**
     * @return array{
     *     period_days: int,
     *     from: string,
     *     to: string,
     *     requests: int,
     *     completed: int,
     *     failed: int,
     *     pending: int,
     *     prompt_tokens: int,
     *     completion_tokens: int,
     *     tokens_used: int,
     *     unsplit_tokens: int,
     *     users: int,
     *     conversations: int,
     *     avg_tokens: int,
     *     success_rate: int,
     *     series: list<array{date: string, requests: int, prompt_tokens: int, completion_tokens: int, tokens_used: int, unsplit_tokens: int}>
     * }
     */
    private function emptySnapshot(Carbon $from, Carbon $to): array
    {
        return [
            'period_days' => self::DAYS,
            'from' => $from->toDateString(),
            'to' => $to->toDateString(),
            'requests' => 0,
            'completed' => 0,
            'failed' => 0,
            'pending' => 0,
            'prompt_tokens' => 0,
            'completion_tokens' => 0,
            'tokens_used' => 0,
            'unsplit_tokens' => 0,
            'users' => 0,
            'conversations' => 0,
            'avg_tokens' => 0,
            'success_rate' => 0,
            'series' => $this->emptySeries($from, $to),
        ];
    }

    /**
     * @return list<array{date: string, requests: int, prompt_tokens: int, completion_tokens: int, tokens_used: int, unsplit_tokens: int}>
     */
    private function series(Carbon $from, Carbon $to): array
    {
        $dayExpression = 'date(created_at)';

        $rows = VedaGeneration::query()
            ->whereBetween('created_at', [$from, $to])
            ->selectRaw("{$dayExpression} as day")
            ->selectRaw('count(*) as requests')
            ->selectRaw('coalesce(sum(prompt_tokens), 0) as prompt_tokens')
            ->selectRaw('coalesce(sum(completion_tokens), 0) as completion_tokens')
            ->selectRaw('coalesce(sum(tokens_used), 0) as tokens_used')
            ->groupByRaw($dayExpression)
            ->get()
            ->keyBy(fn ($row): string => (string) $row->day);

        $series = [];
        $cursor = $from->copy()->startOfDay();

        while ($cursor->lte($to)) {
            $date = $cursor->toDateString();
            $row = $rows->get($date);

            $promptTokens = $this->int($row?->prompt_tokens);
            $completionTokens = $this->int($row?->completion_tokens);
            $tokensUsed = $this->int($row?->tokens_used);

            $series[] = [
                'date' => $date,
                'requests' => $this->int($row?->requests),
                'prompt_tokens' => $promptTokens,
                'completion_tokens' => $completionTokens,
                'tokens_used' => $tokensUsed,
                'unsplit_tokens' => $this->unsplit($tokensUsed, $promptTokens, $completionTokens),
            ];

            $cursor->addDay();
        }

        return $series;
    }

    /**
     * @return list<array{date: string, requests: int, prompt_tokens: int, completion_tokens: int, tokens_used: int, unsplit_tokens: int}>
     */
    private function emptySeries(Carbon $from, Carbon $to): array
    {
        $series = [];
        $cursor = $from->copy()->startOfDay();

        while ($cursor->lte($to)) {
            $series[] = [
                'date' => $cursor->toDateString(),
                'requests' => 0,
                'prompt_tokens' => 0,
                'completion_tokens' => 0,
                'tokens_used' => 0,
                'unsplit_tokens' => 0,
            ];
            $cursor->addDay();
        }

        return $series;
    }

    private function conversationCount(Carbon $from, Carbon $to): int
    {
        if (! Schema::hasTable((new VedaChatHistory)->getTable())) {
            return 0;
        }

        return VedaChatHistory::query()
            ->whereBetween('created_at', [$from, $to])
            ->count();
    }

    private function unsplit(int $tokensUsed, int $promptTokens, int $completionTokens): int
    {
        return max(0, $tokensUsed - $promptTokens - $completionTokens);
    }

    private function int(mixed $value): int
    {
        return (int) $value;
    }
}
