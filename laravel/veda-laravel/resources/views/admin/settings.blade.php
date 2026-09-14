<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Veda settings</title>
    <style>
        :root { color-scheme: light dark; }
        body { font-family: ui-sans-serif, system-ui, sans-serif; margin: 0; background: #0f172a; color: #e2e8f0; }
        main { max-width: 52rem; margin: 2rem auto; padding: 1.5rem; }
        h1 { margin: 0; }
        h2 { margin: 1.5rem 0 0.5rem; font-size: 1.05rem; }
        header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; margin-bottom: 1.5rem; }
        form.card, .card { background: #1e293b; padding: 1.25rem; border-radius: 1rem; margin-bottom: 1rem; }
        label { display: block; margin: 0.85rem 0 0.35rem; font-size: 0.9rem; }
        input, textarea, select { width: 100%; box-sizing: border-box; padding: 0.65rem 0.75rem; border-radius: 0.55rem; border: 1px solid #334155; background: #0f172a; color: inherit; }
        textarea { min-height: 6rem; }
        button, .link-button { padding: 0.65rem 1rem; border: 0; border-radius: 0.55rem; background: #38bdf8; color: #0f172a; font-weight: 600; cursor: pointer; }
        .row { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
        @media (max-width: 720px) { .row { grid-template-columns: 1fr; } }
        .hint { color: #94a3b8; font-size: 0.8rem; margin-top: 0.25rem; }
        .actions { display: flex; gap: 0.75rem; margin-top: 1rem; }
        .checks { display: flex; gap: 1rem; margin-top: 0.5rem; }
        .checks label { display: flex; align-items: center; gap: 0.4rem; margin: 0; }
        .checks input { width: auto; }
    </style>
</head>
<body>
<main>
    <header>
        <h1>Veda settings</h1>
        <form method="post" action="{{ route('veda.admin.logout') }}">
            @csrf
            <button class="link-button" type="submit">Log out</button>
        </form>
    </header>
    <form class="card" method="post" action="{{ route('veda.admin.settings.form') }}">
        @csrf
        <label for="default_model">Default model</label>
        <input id="default_model" name="default_model" value="{{ $settings['default_model'] ?? ($settings['model'] ?? '') }}">
        <p class="hint">Catalog id, for example deepseek-v4-flash-responses.</p>
        <label for="failover">Failover models</label>
        <input id="failover" name="failover" value="{{ implode(', ', $settings['failover'] ?? []) }}">
        <p class="hint">Comma-separated catalog ids.</p>
        <label for="deepseek_key">Shared DeepSeek API key</label>
        <input id="deepseek_key" name="deepseek_key" value="{{ $settings['deepseek']['key'] ?? '' }}" autocomplete="off">
        <label for="max_steps">Max agent steps</label>
        <input id="max_steps" name="max_steps" type="number" min="1" max="200" value="{{ $settings['max_steps'] ?? 30 }}">
        <label>
            <input type="checkbox" name="compaction[enabled]" value="1" @checked($settings['compaction']['enabled'] ?? true)>
            Compaction enabled
        </label>
        <div class="row">
            <div>
                <label for="min_messages">Compaction min messages</label>
                <input id="min_messages" name="compaction[min_messages]" type="number" min="1" value="{{ $settings['compaction']['min_messages'] ?? 40 }}">
            </div>
            <div>
                <label for="keep_tail_messages">Keep tail messages</label>
                <input id="keep_tail_messages" name="compaction[keep_tail_messages]" type="number" min="1" value="{{ $settings['compaction']['keep_tail_messages'] ?? 20 }}">
            </div>
        </div>
        <label for="welcome_message">Welcome message</label>
        <textarea id="welcome_message" name="welcome_message">{{ $settings['welcome_message'] ?? '' }}</textarea>
        <label for="system_prompt">System prompt</label>
        <textarea id="system_prompt" name="system_prompt">{{ $settings['system_prompt'] ?? '' }}</textarea>
        <label for="cors_allowed_origins">CORS origins</label>
        <textarea id="cors_allowed_origins" name="cors_allowed_origins">{{ implode("\n", $settings['cors']['allowed_origins'] ?? []) }}</textarea>
        @php
            $models = array_values($settings['models'] ?? []);
            $models[] = ['id' => '', 'label' => '', 'protocol' => 'responses', 'api_model' => '', 'url' => '', 'key' => '', 'thinking' => false, 'vision' => false, 'aliases' => []];
        @endphp
        @foreach ($models as $index => $model)
            <h2>{{ ($model['id'] ?? '') !== '' ? $model['id'] : 'New model' }}</h2>
            <div class="row">
                <div>
                    <label for="model_id_{{ $index }}">Id</label>
                    <input id="model_id_{{ $index }}" name="models[{{ $index }}][id]" value="{{ $model['id'] ?? '' }}">
                </div>
                <div>
                    <label for="model_label_{{ $index }}">Label</label>
                    <input id="model_label_{{ $index }}" name="models[{{ $index }}][label]" value="{{ $model['label'] ?? '' }}">
                </div>
            </div>
            <div class="row">
                <div>
                    <label for="model_protocol_{{ $index }}">Protocol</label>
                    <select id="model_protocol_{{ $index }}" name="models[{{ $index }}][protocol]">
                        <option value="responses" @selected(($model['protocol'] ?? '') === 'responses')>responses</option>
                        <option value="anthropic" @selected(($model['protocol'] ?? '') === 'anthropic')>anthropic</option>
                    </select>
                </div>
                <div>
                    <label for="model_api_{{ $index }}">API model</label>
                    <input id="model_api_{{ $index }}" name="models[{{ $index }}][api_model]" value="{{ $model['api_model'] ?? '' }}">
                </div>
            </div>
            <label for="model_url_{{ $index }}">URL</label>
            <input id="model_url_{{ $index }}" name="models[{{ $index }}][url]" value="{{ $model['url'] ?? '' }}">
            <label for="model_key_{{ $index }}">API key</label>
            <input id="model_key_{{ $index }}" name="models[{{ $index }}][key]" value="{{ $model['key'] ?? '' }}" autocomplete="off">
            <label for="model_aliases_{{ $index }}">Aliases</label>
            <input id="model_aliases_{{ $index }}" name="models[{{ $index }}][aliases]" value="{{ implode(', ', $model['aliases'] ?? []) }}">
            <input type="hidden" name="models[{{ $index }}][preset]" value="{{ $model['preset'] ?? '' }}">
            <div class="checks">
                <label>
                    <input type="checkbox" name="models[{{ $index }}][thinking]" value="1" @checked($model['thinking'] ?? false)>
                    Thinking
                </label>
                <label>
                    <input type="checkbox" name="models[{{ $index }}][vision]" value="1" @checked($model['vision'] ?? false)>
                    Vision
                </label>
            </div>
        @endforeach
        <div class="actions">
            <button type="submit">Save</button>
        </div>
    </form>
</main>
</body>
</html>
