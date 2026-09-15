<?php

namespace Veda\Laravel\Http\Requests;

use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;
use Illuminate\Validation\Validator;
use Veda\Laravel\Services\VedaAppearance;

class UpdateAdminSettingsRequest extends FormRequest
{
    public function authorize(): bool
    {
        return true;
    }

    protected function prepareForValidation(): void
    {
        $failover = $this->input('failover');
        if (is_string($failover)) {
            $this->merge([
                'failover' => array_values(array_filter(array_map('trim', explode(',', $failover)))),
            ]);
        }

        $origins = $this->input('cors_allowed_origins');
        if (is_string($origins)) {
            $this->merge([
                'cors' => [
                    'allowed_origins' => array_values(array_filter(array_map(
                        'trim',
                        preg_split('/\r\n|\r|\n/', $origins) ?: []
                    ))),
                ],
            ]);
        }

        if ($this->exists('compaction') || $this->has('compaction.enabled')) {
            $compaction = (array) $this->input('compaction', []);
            $compaction['enabled'] = $this->boolean('compaction.enabled');
            $this->merge(['compaction' => $compaction]);
        }

        if ($this->exists('deepseek_key')) {
            $this->merge([
                'deepseek' => [
                    'key' => (string) $this->input('deepseek_key'),
                ],
            ]);
        }

        $models = $this->input('models');
        if (is_array($models)) {
            $models = array_values(array_filter($models, function ($model): bool {
                return is_array($model) && trim((string) ($model['id'] ?? '')) !== '';
            }));

            foreach ($models as $index => $model) {
                $models[$index]['thinking'] = filter_var($model['thinking'] ?? false, FILTER_VALIDATE_BOOLEAN);
                $models[$index]['vision'] = filter_var($model['vision'] ?? false, FILTER_VALIDATE_BOOLEAN);

                if (isset($model['aliases']) && is_string($model['aliases'])) {
                    $models[$index]['aliases'] = array_values(array_filter(array_map(
                        'trim',
                        explode(',', $model['aliases'])
                    )));
                }
            }

            $this->merge(['models' => $models]);
        }

        $mcp = $this->input('mcp');
        if (is_array($mcp) && isset($mcp['mcpServers']) && is_array($mcp['mcpServers'])) {
            foreach ($mcp['mcpServers'] as $id => $server) {
                if (! is_array($server)) {
                    continue;
                }

                if (array_key_exists('disabled', $server)) {
                    $mcp['mcpServers'][$id]['disabled'] = filter_var($server['disabled'], FILTER_VALIDATE_BOOLEAN);
                }
            }

            $this->merge(['mcp' => $mcp]);
        }

        $servers = $this->input('mcp_servers');
        if (is_array($servers)) {
            $servers = array_values(array_filter($servers, function ($server): bool {
                return is_array($server) && trim((string) ($server['id'] ?? '')) !== '';
            }));

            foreach ($servers as $index => $server) {
                $servers[$index]['enabled'] = filter_var($server['enabled'] ?? true, FILTER_VALIDATE_BOOLEAN);
                $auth = strtolower(trim((string) ($server['auth'] ?? 'bearer')));
                $servers[$index]['auth'] = $auth === 'session' ? 'session' : 'bearer';
            }

            $this->merge(['mcp_servers' => $servers]);
        }
    }

    /**
     * @return array<string, mixed>
     */
    public function rules(): array
    {
        return [
            'default_model' => ['nullable', 'string', 'max:128'],
            'model' => ['nullable', 'string', 'max:128'],
            'failover' => ['nullable', 'array'],
            'failover.*' => ['string', 'max:128'],
            'deepseek' => ['nullable', 'array'],
            'deepseek.key' => ['nullable', 'string', 'max:2048'],
            'models' => ['nullable', 'array'],
            'models.*.id' => ['required_with:models', 'string', 'max:128'],
            'models.*.label' => ['required_with:models', 'string', 'max:255'],
            'models.*.protocol' => ['nullable', 'string', Rule::in(['responses', 'anthropic'])],
            'models.*.api_model' => ['nullable', 'string', 'max:128'],
            'models.*.url' => ['nullable', 'string', 'max:2048'],
            'models.*.key' => ['nullable', 'string', 'max:2048'],
            'models.*.thinking' => ['nullable', 'boolean'],
            'models.*.vision' => ['nullable', 'boolean'],
            'models.*.aliases' => ['nullable', 'array'],
            'models.*.aliases.*' => ['string', 'max:128'],
            'models.*.preset' => ['nullable', 'string', 'max:64'],
            'max_steps' => ['nullable', 'integer', 'min:1', 'max:200'],
            'compaction' => ['nullable', 'array'],
            'compaction.enabled' => ['nullable', 'boolean'],
            'compaction.min_messages' => ['nullable', 'integer', 'min:1', 'max:500'],
            'compaction.keep_tail_messages' => ['nullable', 'integer', 'min:1', 'max:500'],
            'cors' => ['nullable', 'array'],
            'cors.allowed_origins' => ['nullable', 'array'],
            'cors.allowed_origins.*' => ['string', 'max:2048'],
            'welcome_message' => ['nullable', 'string', 'max:5000'],
            'system_prompt' => ['nullable', 'string', 'max:20000'],
            'appearance' => ['nullable', 'array'],
            'appearance.preset' => ['nullable', 'string', Rule::in([...VedaAppearance::presetIds(), VedaAppearance::PRESET_CUSTOM, VedaAppearance::PRESET_ROUNDED])],
            'appearance.radius' => ['nullable', 'string', 'max:16'],
            'appearance.tokens' => ['nullable', 'array'],
            'appearance.tokens.*' => ['nullable', 'string', 'max:64'],
            'appearance.dark_tokens' => ['nullable', 'array'],
            'appearance.dark_tokens.*' => ['nullable', 'string', 'max:64'],
            'appearance.launcher' => ['nullable', 'array'],
            'appearance.launcher.label' => ['nullable', 'string', 'max:64'],
            'appearance.launcher.icon' => ['nullable', 'string', Rule::in(VedaAppearance::launcherIconIds())],
            'appearance.launcher.image' => ['nullable', 'string', 'max:'.VedaAppearance::LAUNCHER_IMAGE_MAX_CHARS],
            'mcp' => ['nullable', 'array'],
            'mcp.mcpServers' => ['nullable', 'array'],
            'mcp_servers' => ['nullable', 'array'],
            'mcp_servers.*.id' => ['required_with:mcp_servers', 'string', 'max:128', 'distinct'],
            'mcp_servers.*.label' => ['nullable', 'string', 'max:255'],
            'mcp_servers.*.url' => ['nullable', 'string', 'max:2048'],
            'mcp_servers.*.auth' => ['nullable', 'string', Rule::in(['bearer', 'session'])],
            'mcp_servers.*.token' => ['nullable', 'string', 'max:2048'],
            'mcp_servers.*.enabled' => ['nullable', 'boolean'],
        ];
    }

    public function after(): array
    {
        return [
            function (Validator $validator): void {
                $servers = $this->input('mcp.mcpServers');
                if (! is_array($servers)) {
                    return;
                }

                if ($servers !== [] && array_is_list($servers)) {
                    $validator->errors()->add('mcp.mcpServers', 'object');

                    return;
                }

                foreach ($servers as $id => $server) {
                    if (trim((string) $id) === '' || ! is_array($server)) {
                        $validator->errors()->add('mcp.mcpServers', 'server');

                        continue;
                    }

                    $url = trim((string) ($server['url'] ?? ''));
                    $command = trim((string) ($server['command'] ?? ''));
                    if ($url === '' && $command === '') {
                        $validator->errors()->add('mcp.mcpServers.'.$id, 'endpoint');
                    }
                }
            },
        ];
    }
}
