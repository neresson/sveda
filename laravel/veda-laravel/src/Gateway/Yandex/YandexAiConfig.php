<?php

namespace Veda\Laravel\Gateway\Yandex;

final class YandexAiConfig
{
    public const DEFAULT_CHAT_ENDPOINT = 'https://ai.api.cloud.yandex.net/v1/responses';

    public const DEFAULT_EMBEDDING_ENDPOINT = 'https://llm.api.cloud.yandex.net/foundationModels/v1/textEmbedding';

    /**
     * @param  array<string, mixed>  $config
     */
    public function __construct(protected array $config) {}

    public function chatEndpoint(): string
    {
        $endpoint = trim((string) ($this->config['url'] ?? ''));

        return $endpoint !== '' ? $endpoint : self::DEFAULT_CHAT_ENDPOINT;
    }

    public function embeddingEndpoint(): string
    {
        $endpoint = trim((string) ($this->config['embedding_url'] ?? ''));

        return $endpoint !== '' ? $endpoint : self::DEFAULT_EMBEDDING_ENDPOINT;
    }

    public function folderId(): string
    {
        $id = trim((string) ($this->config['folder_id'] ?? ''));
        if ($id !== '') {
            return $id;
        }

        foreach ([$this->config['models']['text']['default'] ?? null] as $uri) {
            $parsed = $this->parseFolderIdFromUri((string) $uri);
            if ($parsed !== '') {
                return $parsed;
            }
        }

        throw new \RuntimeException(
            'Yandex AI folder id is not configured. Set veda.providers.veda-yandex.folder_id or VEDA_YANDEX_FOLDER_ID.'
        );
    }

    public function modelSlug(?string $model = null): string
    {
        $raw = trim((string) ($model ?? ''));
        if ($raw === '') {
            $raw = trim((string) ($this->config['models']['text']['default'] ?? 'yandexgpt/latest'));
        }

        if (str_starts_with($raw, 'gpt://')) {
            $withoutScheme = substr($raw, 6);
            $slash = strpos($withoutScheme, '/');
            if ($slash !== false) {
                return substr($withoutScheme, $slash + 1);
            }
        }

        return ltrim($raw, '/');
    }

    public function modelUri(?string $model = null): string
    {
        $slug = $this->modelSlug($model);
        if (str_starts_with($slug, 'gpt://')) {
            return $slug;
        }

        return 'gpt://'.$this->folderId().'/'.$slug;
    }

    public function useIamBearer(): bool
    {
        return filter_var($this->config['use_iam_bearer'] ?? false, FILTER_VALIDATE_BOOLEAN);
    }

    public function apiKey(): string
    {
        return trim((string) ($this->config['key'] ?? ''));
    }

    public function embeddingApiKey(): string
    {
        $key = trim((string) ($this->config['embedding_api_key'] ?? ''));

        return $key !== '' ? $key : $this->apiKey();
    }

    /**
     * @return array<string, string>
     */
    public function requestHeaders(string $apiKey, string $authorizationScheme): array
    {
        return [
            'Authorization' => trim($authorizationScheme).' '.$apiKey,
            'Content-Type' => 'application/json',
            'OpenAI-Project' => $this->folderId(),
        ];
    }

    protected function parseFolderIdFromUri(string $uri): string
    {
        if (preg_match('#(?:emb|gpt)://([^/]+)/#', $uri, $matches) === 1) {
            return trim($matches[1]);
        }

        return '';
    }
}
