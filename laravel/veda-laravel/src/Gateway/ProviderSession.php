<?php

namespace Veda\Laravel\Gateway;

class ProviderSession
{
    public function __construct(
        public string $provider,
        public string $model,
        public string $apiKey,
        public string $apiEndpoint,
        public bool $deepseekThinkingEnabled = false,
        public string $authorizationScheme = 'Bearer',
    ) {}
}
