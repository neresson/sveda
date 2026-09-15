<?php

namespace Veda\Laravel\Services;

use Illuminate\Http\Request;
use Illuminate\Support\Facades\Schema;
use Veda\Laravel\Models\VedaGeneration;

class VedaAdminUsage
{
    public const PER_PAGE = 25;

    public function __construct(private VedaModelCatalog $catalog) {}

    /**
     * @return array{
     *     by_model: list<array{model: string, label: string, requests: int, tokens_used: int}>,
     *     requests: array{
     *         data: list<array{id: int, model: string, model_label: string, created_at: ?string, tokens_used: int}>,
     *         current_page: int,
     *         last_page: int,
     *         per_page: int,
     *         total: int,
     *         prev_page_url: ?string,
     *         next_page_url: ?string
     *     }
     * }
     */
    public function snapshot(Request $request): array
    {
        if (! Schema::hasTable((new VedaGeneration)->getTable())) {
            return $this->emptySnapshot();
        }

        $labels = $this->labels();

        return [
            'by_model' => $this->byModel($labels),
            'requests' => $this->requests($request, $labels),
        ];
    }

    /**
     * @return array{
     *     by_model: list<array{model: string, label: string, requests: int, tokens_used: int}>,
     *     requests: array{
     *         data: list<array{id: int, model: string, model_label: string, created_at: ?string, tokens_used: int}>,
     *         current_page: int,
     *         last_page: int,
     *         per_page: int,
     *         total: int,
     *         prev_page_url: ?string,
     *         next_page_url: ?string
     *     }
     * }
     */
    private function emptySnapshot(): array
    {
        return [
            'by_model' => [],
            'requests' => [
                'data' => [],
                'current_page' => 1,
                'last_page' => 1,
                'per_page' => self::PER_PAGE,
                'total' => 0,
                'prev_page_url' => null,
                'next_page_url' => null,
            ],
        ];
    }

    /**
     * @param  array<string, string>  $labels
     * @return list<array{model: string, label: string, requests: int, tokens_used: int}>
     */
    private function byModel(array $labels): array
    {
        $rows = VedaGeneration::query()
            ->selectRaw('coalesce(model, "") as model')
            ->selectRaw('count(*) as requests')
            ->selectRaw('coalesce(sum(tokens_used), 0) as tokens_used')
            ->groupByRaw('coalesce(model, "")')
            ->orderByDesc('tokens_used')
            ->orderByDesc('requests')
            ->get();

        $series = [];
        foreach ($rows as $row) {
            $model = trim((string) $row->model);
            $series[] = [
                'model' => $model,
                'label' => $this->label($model, $labels),
                'requests' => (int) $row->requests,
                'tokens_used' => (int) $row->tokens_used,
            ];
        }

        return $series;
    }

    /**
     * @param  array<string, string>  $labels
     * @return array{
     *     data: list<array{id: int, model: string, model_label: string, created_at: ?string, tokens_used: int}>,
     *     current_page: int,
     *     last_page: int,
     *     per_page: int,
     *     total: int,
     *     prev_page_url: ?string,
     *     next_page_url: ?string
     * }
     */
    private function requests(Request $request, array $labels): array
    {
        $paginator = VedaGeneration::query()
            ->select(['id', 'model', 'created_at', 'tokens_used'])
            ->orderByDesc('id')
            ->paginate(
                perPage: self::PER_PAGE,
                columns: ['id', 'model', 'created_at', 'tokens_used'],
                pageName: 'p',
                page: $request->integer('p') ?: 1,
            )
            ->withPath(route('veda.admin.section', ['page' => 'usage']));

        $data = [];
        foreach ($paginator->items() as $generation) {
            $model = trim((string) $generation->model);
            $data[] = [
                'id' => (int) $generation->id,
                'model' => $model,
                'model_label' => $this->label($model, $labels),
                'created_at' => $generation->created_at?->toIso8601String(),
                'tokens_used' => (int) $generation->tokens_used,
            ];
        }

        return [
            'data' => $data,
            'current_page' => $paginator->currentPage(),
            'last_page' => $paginator->lastPage(),
            'per_page' => $paginator->perPage(),
            'total' => $paginator->total(),
            'prev_page_url' => $paginator->previousPageUrl(),
            'next_page_url' => $paginator->nextPageUrl(),
        ];
    }

    /**
     * @return array<string, string>
     */
    private function labels(): array
    {
        $labels = [];
        foreach ($this->catalog->all() as $definition) {
            $labels[$definition->id] = $definition->label !== '' ? $definition->label : $definition->id;
        }

        return $labels;
    }

    /**
     * @param  array<string, string>  $labels
     */
    private function label(string $model, array $labels): string
    {
        if ($model === '') {
            return '';
        }

        return $labels[$model] ?? $model;
    }
};
