<?php

namespace Veda\Laravel\Http\Requests;

use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Validation\Rule;

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
        ];
    }
}
