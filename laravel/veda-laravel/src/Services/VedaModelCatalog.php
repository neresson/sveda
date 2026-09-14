<?php

namespace Veda\Laravel\Services;

use Veda\Laravel\Enums\VedaProtocol;
use Veda\Laravel\Exceptions\UnknownVedaModelException;
use Veda\Laravel\Support\VedaModelDefinition;

class VedaModelCatalog
{
    public const PROVIDER_PREFIX = 'veda-model:';

    /**
     * @return array<int, VedaModelDefinition>
     */
    public function all(): array
    {
        $definitions = [];

        foreach ($this->rawModels() as $model) {
            $definition = $this->hydrate($model);
            if ($definition !== null) {
                $definitions[] = $definition;
            }
        }

        return $definitions;
    }

    public function find(string $id): ?VedaModelDefinition
    {
        $needle = trim($id);
        if ($needle === '') {
            return null;
        }

        foreach ($this->all() as $model) {
            if ($model->matches($needle)) {
                return $model;
            }
        }

        foreach ($this->all() as $model) {
            if (strtolower($model->apiModel) === strtolower($needle)) {
                return $model;
            }
        }

        return null;
    }

    public function resolve(?string $id): VedaModelDefinition
    {
        $needle = is_string($id) ? trim($id) : '';

        if ($needle === '') {
            return $this->default();
        }

        $found = $this->find($needle);
        if ($found !== null) {
            return $found;
        }

        throw new UnknownVedaModelException($needle);
    }

    public function default(): VedaModelDefinition
    {
        $configured = config('veda.default_model', config('veda.model'));
        $id = is_string($configured) ? trim($configured) : '';

        if ($id !== '') {
            $found = $this->find($id);
            if ($found !== null) {
                return $found;
            }
        }

        $all = $this->all();
        if ($all === []) {
            throw new UnknownVedaModelException;
        }

        return $all[0];
    }

    /**
     * @return array<string, string>
     */
    public function streamProviders(VedaModelDefinition $primary): array
    {
        $chain = [$primary];

        foreach ($this->failoverIds() as $id) {
            $candidate = $this->find($id);
            if ($candidate === null) {
                continue;
            }

            foreach ($chain as $existing) {
                if ($existing->id === $candidate->id) {
                    continue 2;
                }
            }

            $chain[] = $candidate;
        }

        $providers = [];
        foreach ($chain as $model) {
            $providers[$model->laravelProviderName()] = $model->apiModel;
        }

        return $providers;
    }

    /**
     * @return array{provider: string, model: string}
     */
    public function promptTarget(?string $configuredModelId = null): array
    {
        $resolved = $this->resolve($configuredModelId);

        return [
            'provider' => $resolved->laravelProviderName(),
            'model' => $resolved->apiModel,
        ];
    }

    /**
     * @return array<int, array{id: string, label: string, protocol: string, supports_thinking: bool, supportsThinking: bool}>
     */
    public function publicModels(): array
    {
        return array_map(
            fn (VedaModelDefinition $model): array => $model->toPublicArray(),
            $this->all(),
        );
    }

    public function registerIntoAi(): void
    {
        $existing = (array) config('ai.providers', []);

        foreach (array_keys($existing) as $name) {
            if (is_string($name) && str_starts_with($name, self::PROVIDER_PREFIX)) {
                unset($existing[$name]);
            }
        }

        foreach ($this->all() as $model) {
            $existing[$model->laravelProviderName()] = $model->toAiProviderConfig();
        }

        config(['ai.providers' => $existing]);
    }

    /**
     * @return array<int, array<string, mixed>>
     */
    public function settingsModels(): array
    {
        return array_map(
            fn (VedaModelDefinition $model): array => $model->toSettingsArray(),
            $this->all(),
        );
    }

    /**
     * @return array<int, string>
     */
    public function failoverIds(): array
    {
        $raw = config('veda.failover', []);
        if (is_string($raw)) {
            $raw = explode(',', $raw);
        }

        if (! is_array($raw)) {
            return [];
        }

        return array_values(array_filter(array_map(function ($item): string {
            return trim((string) $item);
        }, $raw), fn (string $item): bool => $item !== ''));
    }

    /**
     * @return array<int, mixed>
     */
    protected function rawModels(): array
    {
        $models = config('veda.models', []);

        return is_array($models) ? array_values($models) : [];
    }

    /**
     * @param  mixed  $model
     */
    protected function hydrate(mixed $model): ?VedaModelDefinition
    {
        if (! is_array($model)) {
            return null;
        }

        $id = trim((string) ($model['id'] ?? ''));
        $protocol = VedaProtocol::tryFromMixed($model['protocol'] ?? null);
        $apiModel = trim((string) ($model['api_model'] ?? $model['apiModel'] ?? $id));
        $url = trim((string) ($model['url'] ?? ''));

        if ($id === '' || $protocol === null || $apiModel === '' || $url === '') {
            return null;
        }

        $label = trim((string) ($model['label'] ?? $id));
        $preset = isset($model['preset']) && is_string($model['preset']) && $model['preset'] !== ''
            ? $model['preset']
            : null;

        $key = trim((string) ($model['key'] ?? ''));
        if ($key === '' && $preset === 'deepseek') {
            $key = trim((string) config('veda.deepseek.key', ''));
        }

        $aliases = [];
        foreach ((array) ($model['aliases'] ?? []) as $alias) {
            $alias = trim((string) $alias);
            if ($alias !== '') {
                $aliases[] = $alias;
            }
        }

        $thinking = (bool) ($model['thinking'] ?? $model['supports_thinking'] ?? $model['supportsThinking'] ?? false);
        $vision = (bool) ($model['vision'] ?? $model['supports_vision'] ?? $model['supportsVision'] ?? false);

        return new VedaModelDefinition(
            $id,
            $label !== '' ? $label : $id,
            $protocol,
            $apiModel,
            rtrim($url, '/'),
            $key,
            $thinking,
            $vision,
            $aliases,
            $preset,
        );
    }
}
