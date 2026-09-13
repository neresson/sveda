<?php

namespace Veda\Laravel\Services;

use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\VedaManager;

class ToolCatalogSearchService
{
    public function __construct(
        protected ToolCatalogIndex $index,
        protected EmbeddingService $embeddingService,
    ) {}

    /**
     * @param  array<string, array{name: string, description: string, mode: ToolMode, domain: string}>  $definitions
     * @param  array<int, string>  $domains
     * @return array{tools: array<int, array{name: string, domain: string, mode: string, description: string, score: float}>, backend: string, fingerprint: string}
     */
    public function search(array $definitions, string $query, array $domains = [], int $limit = 8): array
    {
        $limit = max(1, min(24, $limit));
        $query = trim($query);

        $this->index->rebuild($definitions);

        $entries = $this->index->entries();
        if ($domains !== []) {
            $allowed = array_map(fn (string $domain): string => mb_strtolower(trim($domain)), $domains);
            $entries = array_filter(
                $entries,
                fn (array $entry): bool => in_array(mb_strtolower((string) $entry['domain']), $allowed, true)
            );
        }

        $scored = [];
        $queryVector = $this->embeddingService->embed($query);
        $keywordWeight = (float) config('veda.tool_catalog.embedding_keyword_weight', 0.35);
        $keywordWeight = max(0.0, min(1.0, $keywordWeight));

        foreach ($entries as $name => $entry) {
            $semanticScore = 0.0;
            if ($queryVector !== [] && ($entry['embedding'] ?? []) !== []) {
                $semanticScore = EmbeddingVectorMath::cosineSimilarity($queryVector, $entry['embedding']);
            }

            $keywordScore = $this->keywordScore($query, $entry);

            $score = $queryVector !== []
                ? ((1.0 - $keywordWeight) * $semanticScore) + ($keywordWeight * $keywordScore)
                : $keywordScore;

            $scored[] = [
                'name' => $name,
                'domain' => $entry['domain'],
                'mode' => $entry['mode'],
                'description' => $entry['description'],
                'score' => round($score, 6),
            ];
        }

        usort($scored, fn (array $a, array $b): int => $b['score'] <=> $a['score'] ?: strcmp($a['name'], $b['name']));

        $minScore = (float) config('veda.tool_catalog.embedding_min_score', 0.12);
        $filtered = array_values(array_filter(
            $scored,
            fn (array $row): bool => $row['score'] >= $minScore || $this->keywordScore($query, $row) >= 0.4
        ));

        if ($filtered === []) {
            $filtered = array_slice($scored, 0, $limit);
        }

        return [
            'tools' => array_slice($filtered, 0, $limit),
            'backend' => $queryVector !== [] ? 'embeddings+keyword' : 'keyword',
            'fingerprint' => $this->index->fingerprint(),
        ];
    }

    /**
     * @param  array{name: string, domain: string, description: string}|array<string, mixed>  $entry
     */
    protected function keywordScore(string $query, array $entry): float
    {
        $haystack = mb_strtolower($entry['name'].' '.$entry['domain'].' '.$entry['description']);
        $tokens = preg_split('/[^\p{L}\p{N}_]+/u', mb_strtolower($query)) ?: [];
        $tokens = array_values(array_filter($tokens, fn (string $token): bool => mb_strlen($token) >= 2));

        if ($tokens === []) {
            return 0.0;
        }

        $hits = 0;
        foreach ($tokens as $token) {
            if (str_contains($haystack, $token)) {
                $hits++;
            }
        }

        return $hits / count($tokens);
    }

    /**
     * @return array<string, array{name: string, description: string, mode: ToolMode, domain: string}>
     */
    public function definitionsForSearch(VedaManager $manager): array
    {
        return $manager->toolDefinitions();
    }
}
