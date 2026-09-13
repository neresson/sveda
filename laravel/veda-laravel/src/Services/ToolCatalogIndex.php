<?php

namespace Veda\Laravel\Services;

use Veda\Laravel\Enums\ToolMode;

class ToolCatalogIndex
{
    /**
     * @var array<string, array{name: string, domain: string, mode: string, description: string, embedding: array<int, float>}>
     */
    private array $entries = [];

    private string $fingerprint = '';

    public function __construct(
        protected EmbeddingService $embeddingService,
        protected ToolEmbeddingCache $embeddingCache,
    ) {}

    /**
     * @param  array<string, array{name: string, description: string, mode: ToolMode, domain: string}>  $definitions
     */
    public function rebuild(array $definitions): void
    {
        $this->entries = [];
        $this->fingerprint = $this->computeFingerprint($definitions);

        $useEmbeddings = (bool) config('veda.tool_catalog.embeddings_enabled', true)
            && $this->embeddingService->isEnabled();

        $model = $this->embeddingService->model();
        $dimensions = $this->embeddingService->dimensions();
        $onlineFill = (bool) config('veda.tool_catalog.embeddings_online_fill', false);

        foreach ($definitions as $name => $definition) {
            $domain = $definition['domain'] ?? 'other';
            $text = $name.' '.$domain.' '.$definition['description'];

            $embedding = [];
            if ($useEmbeddings) {
                $embedding = $this->embeddingCache->get($model, $dimensions, $text) ?? [];

                if ($embedding === [] && $onlineFill) {
                    $embedding = $this->embeddingService->embed($text);
                    if ($embedding !== []) {
                        $this->embeddingCache->put($model, $dimensions, $text, $embedding);
                    }
                }
            }

            $this->entries[$name] = [
                'name' => $name,
                'domain' => $domain,
                'mode' => $definition['mode']->value,
                'description' => $definition['description'],
                'embedding' => $embedding,
            ];
        }
    }

    public function fingerprint(): string
    {
        return $this->fingerprint;
    }

    /**
     * @return array<string, array{name: string, domain: string, mode: string, description: string, embedding: array<int, float>}>
     */
    public function entries(): array
    {
        return $this->entries;
    }

    public function isEmpty(): bool
    {
        return $this->entries === [];
    }

    /**
     * @param  array<string, array{name: string, description: string, mode: ToolMode, domain: string}>  $definitions
     */
    protected function computeFingerprint(array $definitions): string
    {
        $parts = [];
        foreach ($definitions as $name => $definition) {
            $parts[] = $name.'|'.$definition['description'].'|'.$definition['mode']->value.'|'.($definition['domain'] ?? 'other');
        }
        sort($parts);

        return hash('sha256', implode("\n", $parts));
    }
}
