<?php

namespace Veda\Laravel\Support;

use Veda\Laravel\Enums\VedaProtocol;

final readonly class VedaModelDefinition
{
    /**
     * @param  array<int, string>  $aliases
     */
    public function __construct(
        public string $id,
        public string $label,
        public VedaProtocol $protocol,
        public string $apiModel,
        public string $url,
        public string $key,
        public bool $thinking,
        public bool $vision,
        public array $aliases = [],
        public ?string $preset = null,
    ) {}

    public function laravelProviderName(): string
    {
        return 'veda-model:'.$this->id;
    }

    public function matches(string $id): bool
    {
        $needle = strtolower(trim($id));
        if ($needle === '') {
            return false;
        }

        if ($needle === strtolower($this->id)) {
            return true;
        }

        foreach ($this->aliases as $alias) {
            if (strtolower($alias) === $needle) {
                return true;
            }
        }

        return false;
    }

    /**
     * @return array<string, mixed>
     */
    public function toAiProviderConfig(): array
    {
        return [
            'driver' => $this->protocol->driver(),
            'key' => $this->key,
            'url' => $this->url,
            'store' => false,
            'vision' => $this->vision,
            'models' => [
                'text' => [
                    'default' => $this->apiModel,
                    'cheapest' => $this->apiModel,
                    'smartest' => $this->apiModel,
                ],
            ],
        ];
    }

    /**
     * @return array{id: string, label: string, protocol: string, supports_thinking: bool, supportsThinking: bool}
     */
    public function toPublicArray(): array
    {
        return [
            'id' => $this->id,
            'label' => $this->label,
            'protocol' => $this->protocol->value,
            'supports_thinking' => $this->thinking,
            'supportsThinking' => $this->thinking,
        ];
    }

    /**
     * @return array<string, mixed>
     */
    public function toSettingsArray(): array
    {
        return [
            'id' => $this->id,
            'label' => $this->label,
            'protocol' => $this->protocol->value,
            'api_model' => $this->apiModel,
            'url' => $this->url,
            'key' => $this->key,
            'thinking' => $this->thinking,
            'vision' => $this->vision,
            'aliases' => $this->aliases,
            'preset' => $this->preset,
        ];
    }
}
