<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\Schema;
use Veda\Laravel\Models\VedaSetting;

class VedaSettingsRepository
{
    public const MASK = '••••••••';

    public const CACHE_KEY = 'veda.settings.document';

    /**
     * @return array<string, mixed>
     */
    public function document(): array
    {
        $defaults = $this->defaults();

        if (! $this->tableReady()) {
            return $defaults;
        }

        return Cache::remember(self::CACHE_KEY, 300, function () use ($defaults): array {
            $stored = $this->storedDocument();

            return $stored === null ? $defaults : $this->mergeDocuments($defaults, $stored);
        });
    }

    /**
     * @return array<string, mixed>
     */
    public function maskedDocument(): array
    {
        return $this->mask($this->document());
    }

    /**
     * @return array<string, mixed>
     */
    public function publicDocument(): array
    {
        $document = $this->document();
        $models = [];

        foreach ((array) ($document['models'] ?? []) as $model) {
            if (! is_array($model)) {
                continue;
            }

            $id = trim((string) ($model['id'] ?? ''));
            $label = trim((string) ($model['label'] ?? $id));
            if ($id === '' || $label === '') {
                continue;
            }

            $thinking = (bool) ($model['thinking'] ?? $model['supports_thinking'] ?? $model['supportsThinking'] ?? false);

            $models[] = [
                'id' => $id,
                'label' => $label,
                'protocol' => (string) ($model['protocol'] ?? 'responses'),
                'supports_thinking' => $thinking,
                'supportsThinking' => $thinking,
            ];
        }

        $defaultModel = is_string($document['default_model'] ?? null) ? $document['default_model'] : null;

        return [
            'default_model' => $defaultModel,
            'model' => $defaultModel,
            'models' => $models,
            'welcome_message' => $document['welcome_message'] ?? '',
        ];
    }

    /**
     * @param  array<string, mixed>  $payload
     * @return array<string, mixed>
     */
    public function update(array $payload): array
    {
        $current = $this->document();
        $merged = $this->preserveSecrets($current, $this->mergeDocuments($current, $payload));
        $this->store($merged);
        $this->applyToConfig($merged);

        return $this->mask($merged);
    }

    public function applyToConfig(?array $document = null): void
    {
        if ($document === null) {
            $stored = $this->storedDocument();
            if ($stored === null) {
                return;
            }

            $document = $this->mergeDocuments($this->defaults(), $stored);
        }

        $failover = $this->stringList($document['failover'] ?? []);
        $defaultModel = is_string($document['default_model'] ?? null) && $document['default_model'] !== ''
            ? $document['default_model']
            : (is_string($document['model'] ?? null) ? $document['model'] : (string) config('veda.default_model'));
        $compaction = is_array($document['compaction'] ?? null) ? $document['compaction'] : [];
        $cors = is_array($document['cors'] ?? null) ? $document['cors'] : [];
        $deepseek = is_array($document['deepseek'] ?? null) ? $document['deepseek'] : [];

        config([
            'veda.default_model' => $defaultModel,
            'veda.model' => $defaultModel,
            'veda.failover' => $failover,
            'veda.models' => is_array($document['models'] ?? null) ? $document['models'] : [],
            'veda.deepseek.key' => is_string($deepseek['key'] ?? null) ? $deepseek['key'] : (string) config('veda.deepseek.key', ''),
            'veda.max_steps' => max(1, (int) ($document['max_steps'] ?? config('veda.max_steps', 30))),
            'veda.compaction.enabled' => (bool) ($compaction['enabled'] ?? config('veda.compaction.enabled', true)),
            'veda.compaction.min_messages' => max(1, (int) ($compaction['min_messages'] ?? config('veda.compaction.min_messages', 40))),
            'veda.compaction.keep_tail_messages' => max(1, (int) ($compaction['keep_tail_messages'] ?? config('veda.compaction.keep_tail_messages', 20))),
            'veda.cors.allowed_origins' => $this->stringList($cors['allowed_origins'] ?? config('veda.cors.allowed_origins', [])),
            'veda.welcome_message' => is_string($document['welcome_message'] ?? null) ? $document['welcome_message'] : '',
            'veda.system_prompt' => is_string($document['system_prompt'] ?? null) ? $document['system_prompt'] : '',
        ]);

        app(VedaModelCatalog::class)->registerIntoAi();
    }

    /**
     * @return array<string, mixed>
     */
    public function defaults(): array
    {
        return [
            'default_model' => is_string(config('veda.default_model')) ? config('veda.default_model') : 'deepseek-v4-flash-responses',
            'model' => is_string(config('veda.default_model', config('veda.model'))) ? config('veda.default_model', config('veda.model')) : 'deepseek-v4-flash-responses',
            'failover' => $this->stringList(config('veda.failover', [])),
            'deepseek' => [
                'key' => (string) config('veda.deepseek.key', ''),
            ],
            'models' => $this->normalizeModels(config('veda.models', [])),
            'max_steps' => max(1, (int) config('veda.max_steps', 30)),
            'compaction' => [
                'enabled' => (bool) config('veda.compaction.enabled', true),
                'min_messages' => max(1, (int) config('veda.compaction.min_messages', 40)),
                'keep_tail_messages' => max(1, (int) config('veda.compaction.keep_tail_messages', 20)),
            ],
            'cors' => [
                'allowed_origins' => $this->stringList(config('veda.cors.allowed_origins', [])),
            ],
            'welcome_message' => (string) config('veda.welcome_message', ''),
            'system_prompt' => (string) config('veda.system_prompt', ''),
        ];
    }

    /**
     * @return array<string, mixed>|null
     */
    protected function storedDocument(): ?array
    {
        if (! $this->tableReady()) {
            return null;
        }

        $row = VedaSetting::query()->where('key', VedaSetting::DOCUMENT_KEY)->first();
        if ($row === null || ! is_array($row->value)) {
            return null;
        }

        return $row->value;
    }

    protected function tableReady(): bool
    {
        try {
            return Schema::hasTable((new VedaSetting)->getTable());
        } catch (\Throwable) {
            return false;
        }
    }

    /**
     * @param  array<string, mixed>  $document
     */
    protected function store(array $document): void
    {
        VedaSetting::query()->updateOrCreate(
            ['key' => VedaSetting::DOCUMENT_KEY],
            ['value' => $document],
        );

        Cache::forget(self::CACHE_KEY);
    }

    /**
     * @param  array<string, mixed>  $base
     * @param  array<string, mixed>  $overlay
     * @return array<string, mixed>
     */
    protected function mergeDocuments(array $base, array $overlay): array
    {
        $merged = $base;

        foreach (['default_model', 'model', 'welcome_message', 'system_prompt'] as $field) {
            if (array_key_exists($field, $overlay) && (is_string($overlay[$field]) || $overlay[$field] === null)) {
                $merged[$field] = $overlay[$field];
            }
        }

        if (is_string($merged['default_model'] ?? null) && $merged['default_model'] !== '') {
            $merged['model'] = $merged['default_model'];
        } elseif (is_string($merged['model'] ?? null) && ($merged['default_model'] ?? '') === '') {
            $merged['default_model'] = $merged['model'];
        }

        if (array_key_exists('max_steps', $overlay) && is_numeric($overlay['max_steps'])) {
            $merged['max_steps'] = max(1, (int) $overlay['max_steps']);
        }

        if (array_key_exists('failover', $overlay)) {
            $merged['failover'] = $this->stringList($overlay['failover']);
        }

        if (array_key_exists('models', $overlay)) {
            $merged['models'] = $this->normalizeModels($overlay['models'], $merged['models'] ?? []);
        }

        if (isset($overlay['compaction']) && is_array($overlay['compaction'])) {
            $compaction = is_array($merged['compaction'] ?? null) ? $merged['compaction'] : [];
            if (array_key_exists('enabled', $overlay['compaction'])) {
                $compaction['enabled'] = filter_var($overlay['compaction']['enabled'], FILTER_VALIDATE_BOOLEAN);
            }
            if (array_key_exists('min_messages', $overlay['compaction']) && is_numeric($overlay['compaction']['min_messages'])) {
                $compaction['min_messages'] = max(1, (int) $overlay['compaction']['min_messages']);
            }
            if (array_key_exists('keep_tail_messages', $overlay['compaction']) && is_numeric($overlay['compaction']['keep_tail_messages'])) {
                $compaction['keep_tail_messages'] = max(1, (int) $overlay['compaction']['keep_tail_messages']);
            }
            $merged['compaction'] = $compaction;
        }

        if (isset($overlay['cors']) && is_array($overlay['cors'])) {
            $cors = is_array($merged['cors'] ?? null) ? $merged['cors'] : [];
            if (array_key_exists('allowed_origins', $overlay['cors'])) {
                $cors['allowed_origins'] = $this->stringList($overlay['cors']['allowed_origins']);
            }
            $merged['cors'] = $cors;
        }

        if (isset($overlay['deepseek']) && is_array($overlay['deepseek'])) {
            $deepseek = is_array($merged['deepseek'] ?? null) ? $merged['deepseek'] : [];
            if (array_key_exists('key', $overlay['deepseek'])) {
                $deepseek['key'] = (string) $overlay['deepseek']['key'];
            }
            $merged['deepseek'] = $deepseek;
        }

        return $merged;
    }

    /**
     * @param  array<string, mixed>  $current
     * @param  array<string, mixed>  $next
     * @return array<string, mixed>
     */
    protected function preserveSecrets(array $current, array $next): array
    {
        $currentDeepseek = is_array($current['deepseek'] ?? null) ? $current['deepseek'] : [];
        $nextDeepseek = is_array($next['deepseek'] ?? null) ? $next['deepseek'] : [];
        $incomingDeepseekKey = (string) ($nextDeepseek['key'] ?? '');
        if ($incomingDeepseekKey === self::MASK || $incomingDeepseekKey === '') {
            $nextDeepseek['key'] = (string) ($currentDeepseek['key'] ?? '');
        }
        $next['deepseek'] = $nextDeepseek;

        $currentById = [];
        foreach ((array) ($current['models'] ?? []) as $model) {
            if (is_array($model) && isset($model['id'])) {
                $currentById[(string) $model['id']] = $model;
            }
        }

        $models = [];
        foreach ((array) ($next['models'] ?? []) as $model) {
            if (! is_array($model)) {
                continue;
            }
            $id = (string) ($model['id'] ?? '');
            $incomingKey = (string) ($model['key'] ?? '');
            if ($incomingKey === self::MASK || $incomingKey === '') {
                $model['key'] = (string) ($currentById[$id]['key'] ?? '');
            }
            $models[] = $model;
        }
        $next['models'] = $models;

        return $next;
    }

    /**
     * @param  array<string, mixed>  $document
     * @return array<string, mixed>
     */
    protected function mask(array $document): array
    {
        $deepseek = is_array($document['deepseek'] ?? null) ? $document['deepseek'] : [];
        $deepseekKey = (string) ($deepseek['key'] ?? '');
        $deepseek['key'] = $deepseekKey === '' ? '' : self::MASK;
        $document['deepseek'] = $deepseek;

        $models = [];
        foreach ((array) ($document['models'] ?? []) as $model) {
            if (! is_array($model)) {
                continue;
            }
            $key = (string) ($model['key'] ?? '');
            $model['key'] = $key === '' ? '' : self::MASK;
            $models[] = $model;
        }
        $document['models'] = $models;

        return $document;
    }

    /**
     * @param  array<int, mixed>  $existing
     * @return array<int, array<string, mixed>>
     */
    protected function normalizeModels(mixed $models, array $existing = []): array
    {
        if (! is_array($models)) {
            return [];
        }

        $existingById = [];
        foreach ($existing as $model) {
            if (is_array($model) && isset($model['id'])) {
                $existingById[(string) $model['id']] = $model;
            }
        }

        $normalized = [];
        foreach ($models as $model) {
            if (! is_array($model)) {
                continue;
            }
            $id = trim((string) ($model['id'] ?? ''));
            if ($id === '') {
                continue;
            }

            $base = $existingById[$id] ?? [];
            $merged = array_merge($base, $model);
            $label = trim((string) ($merged['label'] ?? $id));
            $protocol = strtolower(trim((string) ($merged['protocol'] ?? '')));
            $apiModel = trim((string) ($merged['api_model'] ?? $merged['apiModel'] ?? $id));
            $url = rtrim(trim((string) ($merged['url'] ?? '')), '/');

            $aliases = [];
            foreach ((array) ($merged['aliases'] ?? []) as $alias) {
                $alias = trim((string) $alias);
                if ($alias !== '') {
                    $aliases[] = $alias;
                }
            }

            $normalized[] = [
                'id' => $id,
                'label' => $label !== '' ? $label : $id,
                'protocol' => in_array($protocol, ['responses', 'anthropic'], true) ? $protocol : 'responses',
                'api_model' => $apiModel !== '' ? $apiModel : $id,
                'url' => $url,
                'key' => (string) ($merged['key'] ?? ''),
                'thinking' => (bool) ($merged['thinking'] ?? $merged['supports_thinking'] ?? $merged['supportsThinking'] ?? false),
                'vision' => (bool) ($merged['vision'] ?? $merged['supports_vision'] ?? $merged['supportsVision'] ?? false),
                'aliases' => $aliases,
                'preset' => isset($merged['preset']) && is_string($merged['preset']) && $merged['preset'] !== ''
                    ? $merged['preset']
                    : null,
            ];
        }

        return $normalized;
    }

    /**
     * @return array<int, string>
     */
    protected function stringList(mixed $value): array
    {
        if (is_string($value)) {
            $value = explode(',', $value);
        }

        if (! is_array($value)) {
            return [];
        }

        return array_values(array_filter(array_map(function ($item): string {
            return trim((string) $item);
        }, $value), fn (string $item): bool => $item !== ''));
    }
}
