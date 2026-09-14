<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\Schema;
use Veda\Laravel\Models\VedaSetting;

class VedaAdminAccess
{
    public const SETTING_KEY = 'admin_api_key';

    public function configuredKey(): string
    {
        $fromEnv = trim((string) config('veda.admin.api_key', ''));
        if ($fromEnv !== '') {
            return $fromEnv;
        }

        return $this->storedKey();
    }

    public function isConfigured(): bool
    {
        return $this->configuredKey() !== '';
    }

    public function matches(string $plain): bool
    {
        $configured = $this->configuredKey();
        $plain = trim($plain);

        return $configured !== '' && $plain !== '' && hash_equals($configured, $plain);
    }

    public function store(string $plain): void
    {
        $plain = trim($plain);
        if ($plain === '') {
            return;
        }

        VedaSetting::query()->updateOrCreate(
            ['key' => self::SETTING_KEY],
            ['value' => ['secret' => $plain]],
        );
    }

    protected function storedKey(): string
    {
        try {
            if (! Schema::hasTable((new VedaSetting)->getTable())) {
                return '';
            }
        } catch (\Throwable) {
            return '';
        }

        $row = VedaSetting::query()->where('key', self::SETTING_KEY)->first();
        $secret = is_array($row?->value) ? ($row->value['secret'] ?? '') : '';

        return is_string($secret) ? trim($secret) : '';
    }
}
