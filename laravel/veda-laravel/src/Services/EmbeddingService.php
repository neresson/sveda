<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Log;
use Laravel\Ai\Embeddings;

class EmbeddingService
{
    public const DEFAULT_MODEL = 'text-embedding-3-small';

    public function isEnabled(): bool
    {
        return (bool) config('veda.embeddings.enabled', true);
    }

    public function model(): string
    {
        $model = config('veda.embeddings.model', self::DEFAULT_MODEL);

        return is_string($model) && trim($model) !== '' ? $model : self::DEFAULT_MODEL;
    }

    public function dimensions(): int
    {
        return max(16, (int) config('veda.embeddings.dimensions', 1536));
    }

    /**
     * @return array<int, float>
     */
    public function embed(string $text): array
    {
        if (! $this->isEnabled()) {
            return [];
        }

        $text = trim($text);
        if ($text === '') {
            return [];
        }

        $text = mb_substr($text, 0, max(500, (int) config('veda.embeddings.max_input_chars', 12000)));

        try {
            $response = Embeddings::for([$text])
                ->dimensions($this->dimensions())
                ->generate($this->providerName(), $this->model());

            $vector = $response->embeddings[0] ?? null;

            if (! is_array($vector)) {
                return [];
            }

            return array_map('floatval', array_values($vector));
        } catch (\Throwable $e) {
            Log::warning('veda.embedding_failed', ['message' => $e->getMessage()]);

            return [];
        }
    }

    public function providerName(): ?string
    {
        $provider = config('veda.embeddings.provider');

        return is_string($provider) && trim($provider) !== '' ? $provider : null;
    }
}
