<?php

namespace Veda\Laravel\Gateway\Yandex;

use Illuminate\Contracts\Events\Dispatcher;
use Illuminate\Support\Facades\Http;
use Laravel\Ai\Contracts\Gateway\EmbeddingGateway;
use Laravel\Ai\Contracts\Providers\EmbeddingProvider;
use Laravel\Ai\Responses\Data\Meta;
use Laravel\Ai\Responses\EmbeddingsResponse;

class YandexEmbeddingGateway implements EmbeddingGateway
{
    protected YandexAiConfig $yandexConfig;

    /**
     * @param  array<string, mixed>  $config
     */
    public function __construct(protected Dispatcher $events, protected array $config)
    {
        $this->yandexConfig = new YandexAiConfig($config);
    }

    public function generateEmbeddings(EmbeddingProvider $provider, string $model, array $inputs, int $dimensions, int $timeout = 30, array $providerOptions = []): EmbeddingsResponse
    {
        $apiKey = $this->yandexConfig->embeddingApiKey();
        $authorizationScheme = filter_var($this->config['embedding_iam_token'] ?? false, FILTER_VALIDATE_BOOLEAN)
            ? 'Bearer'
            : 'Api-Key';

        $embeddings = [];
        $totalTokens = 0;

        foreach ($inputs as $input) {
            $body = [
                'modelUri' => $model,
                'text' => $input,
            ];

            $response = Http::timeout($timeout)
                ->withHeaders($this->yandexConfig->requestHeaders($apiKey, $authorizationScheme))
                ->post($this->yandexConfig->embeddingEndpoint(), $body)
                ->throw();

            $data = $response->json();
            $embeddings[] = $data['embedding'];
            $totalTokens += $data['numTokens'] ?? 0;
        }

        return new EmbeddingsResponse(
            embeddings: $embeddings,
            tokens: $totalTokens,
            meta: new Meta(
                id: '',
                model: $model,
            )
        );
    }
}
