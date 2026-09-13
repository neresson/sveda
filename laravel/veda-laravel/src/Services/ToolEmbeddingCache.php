<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Cache;

class ToolEmbeddingCache
{
    /**
     * @return array<int, float>|null
     */
    public function get(string $model, int $dimensions, string $text): ?array
    {
        $cached = Cache::get($this->key($model, $dimensions, $text));

        if (! is_array($cached)) {
            return null;
        }

        return array_map('floatval', array_values($cached));
    }

    /**
     * @param  array<int, float>  $vector
     */
    public function put(string $model, int $dimensions, string $text, array $vector): void
    {
        $days = max(1, (int) config('veda.tool_catalog.embedding_cache_days', 30));

        Cache::put($this->key($model, $dimensions, $text), array_values($vector), now()->addDays($days));
    }

    public function key(string $model, int $dimensions, string $text): string
    {
        $normalized = mb_strtolower(trim(preg_replace('/\s+/u', ' ', $text) ?? $text));

        return 'veda:tool-emb:v1:'.hash('sha256', $model.'|'.$dimensions.'|'.$normalized);
    }
}
