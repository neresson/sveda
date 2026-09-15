<?php

namespace Veda\Laravel\Services;

use Closure;

class McpCatalog
{
    /**
     * @param  Closure(string): (?string)|null  $env
     */
    public function __construct(
        protected ?Closure $env = null,
        protected string $home = '',
        protected string $workspace = '',
    ) {
        $this->env ??= function (string $name): ?string {
            $value = getenv($name);
            if (is_string($value) && $value !== '') {
                return $value;
            }

            $fallback = env($name);

            return is_string($fallback) ? $fallback : null;
        };

        if ($this->home === '') {
            $home = getenv('HOME') ?: ($_SERVER['HOME'] ?? '');
            $this->home = is_string($home) ? $home : '';
        }

        if ($this->workspace === '') {
            $this->workspace = function_exists('base_path') ? base_path() : (string) getcwd();
        }
    }

    /**
     * @return array{mcpServers: array<string, array<string, mixed>>}
     */
    public function empty(): array
    {
        return ['mcpServers' => []];
    }

    /**
     * @param  array<string, mixed>  $existing
     * @return array{mcpServers: array<string, array<string, mixed>>}
     */
    public function normalize(mixed $mcp, array $existing = []): array
    {
        if (! is_array($mcp)) {
            return $this->empty();
        }

        $servers = $mcp['mcpServers'] ?? [];
        if (! is_array($servers)) {
            return $this->empty();
        }

        $existingServers = is_array($existing['mcpServers'] ?? null) ? $existing['mcpServers'] : [];
        $normalized = [];

        foreach ($servers as $id => $server) {
            if (! is_array($server)) {
                continue;
            }

            $id = trim((string) $id);
            if ($id === '') {
                continue;
            }

            $base = is_array($existingServers[$id] ?? null) ? $existingServers[$id] : [];
            $normalized[$id] = $this->normalizeServer(array_merge($base, $server));
        }

        return ['mcpServers' => $normalized];
    }

    /**
     * @return array{mcpServers: array<string, array<string, mixed>>}
     */
    public function fromLegacyServers(mixed $servers): array
    {
        $mcpServers = [];

        foreach ((array) $servers as $server) {
            if (! is_array($server)) {
                continue;
            }

            $id = trim((string) ($server['id'] ?? ''));
            if ($id === '') {
                continue;
            }

            $entry = $this->normalizeServer($server);
            $auth = strtolower(trim((string) ($server['auth'] ?? '')));
            $token = trim((string) ($server['token'] ?? ''));
            if ($auth !== 'session' && $token !== '') {
                $headers = is_array($entry['headers'] ?? null) ? $entry['headers'] : [];
                $headers['Authorization'] = 'Bearer '.$token;
                $entry['headers'] = $headers;
            }

            if (array_key_exists('enabled', $server) && ! filter_var($server['enabled'], FILTER_VALIDATE_BOOLEAN)) {
                $entry['disabled'] = true;
            }

            unset($entry['id'], $entry['label'], $entry['token'], $entry['auth'], $entry['enabled']);
            $mcpServers[$id] = $entry;
        }

        return ['mcpServers' => $mcpServers];
    }

    /**
     * @param  array<string, mixed>  $mcp
     * @return array{mcpServers: array<string, array<string, mixed>>}
     */
    public function mask(array $mcp): array
    {
        $servers = [];

        foreach ((array) ($mcp['mcpServers'] ?? []) as $id => $server) {
            if (! is_array($server)) {
                continue;
            }

            $servers[(string) $id] = $this->maskServer($server);
        }

        return ['mcpServers' => $servers];
    }

    /**
     * @param  array<string, mixed>  $current
     * @param  array<string, mixed>  $next
     * @return array{mcpServers: array<string, array<string, mixed>>}
     */
    public function preserveSecrets(array $current, array $next): array
    {
        $currentServers = is_array($current['mcpServers'] ?? null) ? $current['mcpServers'] : [];
        $nextServers = is_array($next['mcpServers'] ?? null) ? $next['mcpServers'] : [];

        foreach ($nextServers as $id => $server) {
            if (! is_array($server)) {
                continue;
            }

            $existing = is_array($currentServers[$id] ?? null) ? $currentServers[$id] : [];
            $nextServers[$id] = $this->preserveServerSecrets($existing, $server);
        }

        return ['mcpServers' => $nextServers];
    }

    public function suggestServerId(string $url): string
    {
        $host = strtolower((string) parse_url($url, PHP_URL_HOST));
        $host = preg_replace('/^www\./', '', $host) ?? $host;
        $host = preg_replace('/^mcp\./', '', $host) ?? $host;
        $label = explode('.', $host)[0] ?? '';
        $slug = trim((string) preg_replace('/[^a-z0-9]+/', '_', $label), '_');

        return $slug !== '' ? $slug : 'mcp';
    }

    public function interpolate(string $value): string
    {
        $env = $this->env;
        $interpolated = preg_replace_callback(
            '/\$\{env:([^}]+)\}/',
            function (array $matches) use ($env): string {
                $resolved = $env(trim($matches[1]));

                return is_string($resolved) ? $resolved : '';
            },
            $value,
        );

        $value = is_string($interpolated) ? $interpolated : $value;

        return strtr($value, [
            '${workspaceFolderBasename}' => basename($this->workspace),
            '${workspaceFolder}' => $this->workspace,
            '${userHome}' => $this->home,
            '${pathSeparator}' => DIRECTORY_SEPARATOR,
            '${/}' => DIRECTORY_SEPARATOR,
        ]);
    }

    /**
     * @param  array<string, mixed>  $map
     * @return array<string, string>
     */
    public function interpolateMap(array $map): array
    {
        $out = [];

        foreach ($map as $key => $value) {
            if (is_string($value)) {
                $out[(string) $key] = $this->interpolate($value);
            }
        }

        return $out;
    }

    /**
     * @return array<string, string>
     */
    public function loadEnvFile(string $path): array
    {
        if ($path === '' || ! is_file($path) || ! is_readable($path)) {
            return [];
        }

        $contents = file_get_contents($path);
        if (! is_string($contents) || $contents === '') {
            return [];
        }

        $contents = preg_replace('/^\xEF\xBB\xBF/', '', $contents) ?? $contents;
        $env = [];

        foreach (preg_split('/\r\n|\r|\n/', $contents) ?: [] as $line) {
            $line = trim($line);
            if ($line === '' || str_starts_with($line, '#')) {
                continue;
            }

            if (str_starts_with(strtolower($line), 'export ')) {
                $line = trim(substr($line, 7));
            }

            $eq = strpos($line, '=');
            if ($eq === false || $eq === 0) {
                continue;
            }

            $key = trim(substr($line, 0, $eq));
            $value = trim(substr($line, $eq + 1));
            if ($key === '') {
                continue;
            }

            if (
                strlen($value) >= 2
                && (
                    (str_starts_with($value, '"') && str_ends_with($value, '"'))
                    || (str_starts_with($value, "'") && str_ends_with($value, "'"))
                )
            ) {
                $value = substr($value, 1, -1);
            }

            $env[$key] = $this->interpolate($value);
        }

        return $env;
    }

    /**
     * @param  array<string, mixed>  $server
     * @return array<string, mixed>
     */
    protected function normalizeServer(array $server): array
    {
        $entry = [];
        $url = rtrim(trim((string) ($server['url'] ?? '')), '/');
        $command = trim((string) ($server['command'] ?? ''));

        if ($url !== '') {
            $entry['url'] = $url;
        }

        if ($command !== '') {
            $entry['command'] = $command;
        }

        $type = trim((string) ($server['type'] ?? ''));
        if ($type !== '') {
            $entry['type'] = $type;
        }

        $args = $this->stringList($server['args'] ?? null);
        if ($args !== []) {
            $entry['args'] = $args;
        }

        $headers = $this->stringMap($server['headers'] ?? null);
        if ($headers !== []) {
            $entry['headers'] = $headers;
        }

        $env = $this->stringMap($server['env'] ?? null);
        if ($env !== []) {
            $entry['env'] = $env;
        }

        $envFile = trim((string) ($server['envFile'] ?? ''));
        if ($envFile !== '') {
            $entry['envFile'] = $envFile;
        }

        $cwd = trim((string) ($server['cwd'] ?? ''));
        if ($cwd !== '') {
            $entry['cwd'] = $cwd;
        }

        if (isset($server['auth']) && is_array($server['auth'])) {
            $entry['auth'] = $server['auth'];
        }

        if (filter_var($server['disabled'] ?? false, FILTER_VALIDATE_BOOLEAN)) {
            $entry['disabled'] = true;
        }

        return $entry;
    }

    /**
     * @param  array<string, mixed>  $server
     * @return array<string, mixed>
     */
    protected function maskServer(array $server): array
    {
        if (isset($server['headers']) && is_array($server['headers'])) {
            foreach ($server['headers'] as $name => $value) {
                $server['headers'][$name] = $this->maskNamedValue((string) $name, (string) $value);
            }
        }

        if (isset($server['env']) && is_array($server['env'])) {
            foreach ($server['env'] as $name => $value) {
                $value = (string) $value;
                $server['env'][$name] = $this->isSecretValue($value) ? VedaSettingsRepository::MASK : $value;
            }
        }

        if (isset($server['auth']) && is_array($server['auth'])) {
            foreach (['CLIENT_SECRET', 'client_secret'] as $key) {
                $secret = $server['auth'][$key] ?? null;
                if (is_string($secret) && $this->isSecretValue($secret)) {
                    $server['auth'][$key] = VedaSettingsRepository::MASK;
                }
            }
        }

        return $server;
    }

    /**
     * @param  array<string, mixed>  $existing
     * @param  array<string, mixed>  $server
     * @return array<string, mixed>
     */
    protected function preserveServerSecrets(array $existing, array $server): array
    {
        if (isset($server['headers']) && is_array($server['headers'])) {
            $current = is_array($existing['headers'] ?? null) ? $existing['headers'] : [];
            foreach ($server['headers'] as $name => $value) {
                if ($this->isMasked((string) $value)) {
                    $server['headers'][$name] = (string) ($current[$name] ?? '');
                }
            }
            $server['headers'] = $this->stringMap($server['headers']);
            if ($server['headers'] === []) {
                unset($server['headers']);
            }
        }

        if (isset($server['env']) && is_array($server['env'])) {
            $current = is_array($existing['env'] ?? null) ? $existing['env'] : [];
            foreach ($server['env'] as $name => $value) {
                if ($this->isMasked((string) $value) || trim((string) $value) === '') {
                    $server['env'][$name] = (string) ($current[$name] ?? '');
                }
            }
            $server['env'] = $this->stringMap($server['env']);
            if ($server['env'] === []) {
                unset($server['env']);
            }
        }

        if (isset($server['auth']) && is_array($server['auth'])) {
            $current = is_array($existing['auth'] ?? null) ? $existing['auth'] : [];
            foreach (['CLIENT_SECRET', 'client_secret'] as $key) {
                $value = $server['auth'][$key] ?? null;
                if (is_string($value) && ($this->isMasked($value) || $value === '')) {
                    $server['auth'][$key] = $current[$key] ?? '';
                }
            }
        }

        return $server;
    }

    protected function maskNamedValue(string $name, string $value): string
    {
        if (strcasecmp($name, 'Authorization') === 0 && $this->isSecretValue($value)) {
            return VedaSettingsRepository::MASK;
        }

        return $value;
    }

    protected function isSecretValue(string $value): bool
    {
        return $value !== '' && ! str_contains($value, '${');
    }

    protected function isMasked(string $value): bool
    {
        return $value === VedaSettingsRepository::MASK;
    }

    /**
     * @return array<int, string>
     */
    protected function stringList(mixed $value): array
    {
        if (! is_array($value)) {
            return [];
        }

        $items = [];
        foreach ($value as $item) {
            $item = trim((string) $item);
            if ($item !== '') {
                $items[] = $item;
            }
        }

        return $items;
    }

    /**
     * @return array<string, string>
     */
    protected function stringMap(mixed $value): array
    {
        if (! is_array($value)) {
            return [];
        }

        $map = [];
        foreach ($value as $key => $item) {
            $key = trim((string) $key);
            if ($key === '' || ! is_scalar($item)) {
                continue;
            }

            $item = trim((string) $item);
            if ($item !== '') {
                $map[$key] = $item;
            }
        }

        return $map;
    }
}
