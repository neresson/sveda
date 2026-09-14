<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Veda admin</title>
    <style>
        :root { color-scheme: light dark; }
        body { font-family: ui-sans-serif, system-ui, sans-serif; margin: 0; background: #0f172a; color: #e2e8f0; }
        main { max-width: 28rem; margin: 12vh auto; padding: 2rem; background: #1e293b; border-radius: 1rem; }
        h1 { margin: 0 0 0.5rem; font-size: 1.5rem; }
        p { margin: 0 0 1.5rem; color: #94a3b8; }
        label { display: block; margin-bottom: 0.4rem; font-size: 0.9rem; }
        input { width: 100%; box-sizing: border-box; padding: 0.7rem 0.8rem; border-radius: 0.6rem; border: 1px solid #334155; background: #0f172a; color: inherit; }
        button { margin-top: 1rem; width: 100%; padding: 0.75rem; border: 0; border-radius: 0.6rem; background: #38bdf8; color: #0f172a; font-weight: 600; cursor: pointer; }
        .error { color: #fca5a5; margin: 0 0 1rem; }
    </style>
</head>
<body>
<main>
    <h1>Veda</h1>
    <p>Enter the admin key to manage this sidecar.</p>
    @if ($errors->any())
        <p class="error">{{ $errors->first() }}</p>
    @endif
    <form method="post" action="{{ route('veda.admin.login') }}">
        @csrf
        <label for="key">Admin key</label>
        <input id="key" name="key" type="password" autocomplete="current-password" required>
        <button type="submit">Open settings</button>
    </form>
</main>
</body>
</html>
