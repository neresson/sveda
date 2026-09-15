<?php

namespace Veda\Laravel\CodeIndex;

use GuzzleHttp\Client;
use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Facades\RateLimiter;
use Psr\Http\Message\ResponseInterface;

class CodeIndexEmbeddingService
{
    protected Client $client;

    public function __construct()
    {
        $this->client = new Client([
            'timeout' => 120,
            'connect_timeout' => 15,
            'http_errors' => false,
            'headers' => [
                'Content-Type' => 'application/json',
            ],
        ]);
    }

    public function isConfigured(): bool
    {
        return $this->apiKey() !== '' && $this->documentModelUri() !== '' && $this->queryModelUri() !== '';
    }

    public function isAvailable(): bool
    {
        return $this->isConfigured() && ! Cache::has($this->circuitKey());
    }

    /**
     * @return list<float>|null
     */
    public function createDocumentEmbedding(string $text, int $maxRetries = 3): ?array
    {
        if (! $this->isAvailable()) {
            return null;
        }

        return $this->requestEmbedding($text, $this->documentModelUri(), $maxRetries);
    }

    /**
     * @return list<float>|null
     */
    public function createQueryEmbedding(string $text, int $maxRetries = 3): ?array
    {
        if (! $this->isAvailable()) {
            return null;
        }

        return $this->requestEmbedding($text, $this->queryModelUri(), $maxRetries);
    }

    public function clampEmbeddingInput(string $text): string
    {
        $max = (int) config('veda.embeddings.max_input_chars', 2000);
        $max = max(1024, $max);
        $text = trim($text);
        if (mb_strlen($text) <= $max) {
            return $text;
        }

        return mb_substr($text, 0, $max);
    }

    /**
     * @return list<float>|null
     */
    protected function requestEmbedding(string $text, string $modelUri, int $maxRetries): ?array
    {
        $key = $this->apiKey();
        $text = $this->clampEmbeddingInput($text);
        if ($key === '' || $modelUri === '' || $text === '') {
            return null;
        }

        $endpoint = (string) config('veda.embeddings.endpoint', 'https://llm.api.cloud.yandex.net/foundationModels/v1/textEmbedding');
        $expectedDim = (int) config('veda.code_index.embedding_vector_dimensions', config('veda.embeddings.dimensions', 256));
        $body = [
            'modelUri' => $modelUri,
            'text' => $text,
        ];
        $dim = (int) config('veda.embeddings.dimensions', 0);
        if ($dim > 0) {
            $body['dim'] = (string) $dim;
        }

        $headers = $this->authorizationHeader($key);
        $failures = 0;
        $iterations = 0;
        $maxIterations = max(40, $maxRetries * 15);

        while ($iterations < $maxIterations) {
            $iterations++;
            $this->acquireEmbeddingRateLimitSlot();

            try {
                $response = $this->client->post($endpoint, [
                    'headers' => $headers,
                    'json' => $body,
                ]);
                $status = $response->getStatusCode();
                $raw = (string) $response->getBody();
                $data = json_decode($raw, true);

                if ($status === 200 && is_array($data) && isset($data['embedding']) && is_array($data['embedding'])) {
                    $embedding = array_map(static fn ($v): float => (float) $v, $data['embedding']);
                    if ($expectedDim > 0 && count($embedding) !== $expectedDim) {
                        Log::error('veda.embedding.dimension_mismatch', [
                            'expected' => $expectedDim,
                            'actual' => count($embedding),
                            'model_uri' => $modelUri,
                        ]);
                        $failures++;
                        if ($failures >= $maxRetries) {
                            return null;
                        }
                        usleep(500_000);

                        continue;
                    }

                    return $embedding;
                }

                if ($status === 400 && str_contains(mb_strtolower($raw), 'token')) {
                    Log::warning('veda.embedding.input_too_large', [
                        'model_uri' => $modelUri,
                        'chars' => mb_strlen($text),
                    ]);

                    return null;
                }

                if ($status === 429) {
                    Log::warning('veda.embedding.rate_limited', [
                        'model_uri' => $modelUri,
                    ]);
                    $this->sleepAfterRateLimitResponse($response, $iterations);

                    continue;
                }

                $failures++;
                Log::error('veda.embedding.error', [
                    'failures' => $failures,
                    'status' => $status,
                    'model_uri' => $modelUri,
                    'response' => mb_substr($raw, 0, 2000),
                ]);

                if ($failures >= $maxRetries) {
                    if ($status >= 500) {
                        $this->openCircuit();
                    }

                    return null;
                }
                usleep(500_000);
            } catch (\Throwable $e) {
                $failures++;
                Log::error('veda.embedding.exception', [
                    'failures' => $failures,
                    'endpoint' => $endpoint,
                    'model_uri' => $modelUri,
                    'error' => $e->getMessage(),
                ]);
                if ($failures >= $maxRetries) {
                    $this->openCircuit();

                    return null;
                }
                usleep(500_000);
            }
        }

        return null;
    }

    protected function acquireEmbeddingRateLimitSlot(): void
    {
        $decay = max(1, (int) config('veda.embeddings.rate_decay_seconds', 2));
        $maxAttempts = max(1, min(30, (int) config('veda.embeddings.max_requests_per_decay', 9)));
        $limitKey = (string) config('veda.embeddings.rate_limiter_key', 'veda_text_embedding');

        for ($i = 0; $i < 600; $i++) {
            if (RateLimiter::attempt($limitKey, $maxAttempts, fn () => true, $decay)) {
                return;
            }
            usleep(80_000);
        }
    }

    protected function sleepAfterRateLimitResponse(ResponseInterface $response, int $iteration): void
    {
        $retryHeaders = $response->getHeader('Retry-After');
        if ($retryHeaders !== []) {
            $seconds = (int) $retryHeaders[0];
            if ($seconds > 0) {
                sleep(min(30, $seconds));

                return;
            }
        }

        $base = 1_000_000 * (1 + min(7, (int) floor(($iteration - 1) / 2)));
        $micros = min(8_000_000, max(1_000_000, $base));
        usleep($micros);
    }

    /**
     * @return array<string, string>
     */
    protected function authorizationHeader(string $key): array
    {
        if (config('veda.embeddings.use_iam_bearer', false)) {
            return ['Authorization' => 'Bearer '.$key];
        }

        return ['Authorization' => 'Api-Key '.$key];
    }

    protected function circuitKey(): string
    {
        return 'veda.embeddings.circuit_open';
    }

    protected function openCircuit(): void
    {
        $minutes = max(1, (int) config('veda.embeddings.circuit_open_minutes', 10));
        Cache::put($this->circuitKey(), 1, now()->addMinutes($minutes));
        Log::warning('veda.embedding.circuit_open', [
            'minutes' => $minutes,
        ]);
    }

    protected function apiKey(): string
    {
        return trim((string) config('veda.embeddings.api_key', ''));
    }

    protected function documentModelUri(): string
    {
        return trim((string) config('veda.embeddings.document_model_uri', ''));
    }

    protected function queryModelUri(): string
    {
        $uri = trim((string) config('veda.embeddings.query_model_uri', ''));
        if ($uri === '') {
            $uri = $this->documentModelUri();
        }

        return $uri;
    }
}
