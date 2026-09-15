<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Services\McpCatalog;
use Veda\Laravel\Services\VedaSettingsRepository;
use Veda\Laravel\Tests\TestCase;

class McpCatalogTest extends TestCase
{
    public function test_interpolates_cursor_variables(): void
    {
        $catalog = new McpCatalog(
            env: fn (string $name): ?string => $name === 'API_KEY' ? 'secret' : null,
            home: '/home/veda',
            workspace: '/srv/sidecar',
        );

        $this->assertSame('Bearer secret', $catalog->interpolate('Bearer ${env:API_KEY}'));
        $this->assertSame('/home/veda/.mcp', $catalog->interpolate('${userHome}/.mcp'));
        $this->assertSame('/srv/sidecar/tools/server.py', $catalog->interpolate('${workspaceFolder}/tools/server.py'));
        $this->assertSame('sidecar', $catalog->interpolate('${workspaceFolderBasename}'));
        $this->assertSame('/', $catalog->interpolate('${pathSeparator}'));
        $this->assertSame('/', $catalog->interpolate('${/}'));
    }

    public function test_suggests_server_id_from_mcp_host(): void
    {
        $catalog = new McpCatalog;

        $this->assertSame('vkusvill', $catalog->suggestServerId('https://mcp.vkusvill.ru/mcp'));
        $this->assertSame('docs', $catalog->suggestServerId('https://docs.example.test/mcp'));
        $this->assertSame('mcp', $catalog->suggestServerId('not-a-url'));
    }

    public function test_missing_env_variable_becomes_empty_string(): void
    {
        $catalog = new McpCatalog(env: fn (string $name): ?string => null);

        $this->assertSame('Bearer ', $catalog->interpolate('Bearer ${env:MISSING}'));
    }

    public function test_migrates_legacy_server_list_to_mcp_json(): void
    {
        $catalog = new McpCatalog;

        $mcp = $catalog->fromLegacyServers([
            [
                'id' => 'lms',
                'label' => 'LMS',
                'url' => 'http://lms.test/mcp/veda',
                'auth' => 'session',
                'enabled' => true,
            ],
            [
                'id' => 'docs',
                'label' => 'Docs',
                'url' => 'http://docs.test/mcp',
                'token' => 'docs-secret',
                'enabled' => false,
            ],
        ]);

        $this->assertSame([
            'mcpServers' => [
                'lms' => [
                    'url' => 'http://lms.test/mcp/veda',
                ],
                'docs' => [
                    'url' => 'http://docs.test/mcp',
                    'headers' => [
                        'Authorization' => 'Bearer docs-secret',
                    ],
                    'disabled' => true,
                ],
            ],
        ], $mcp);
    }

    public function test_masks_header_and_env_secrets(): void
    {
        $catalog = new McpCatalog;
        $masked = $catalog->mask([
            'mcpServers' => [
                'docs' => [
                    'url' => 'https://docs.example.test/mcp',
                    'headers' => [
                        'Authorization' => 'Bearer docs-secret',
                        'X-Tenant' => 'acme',
                    ],
                    'env' => [
                        'API_KEY' => 'process-secret',
                    ],
                ],
            ],
        ]);

        $this->assertSame(VedaSettingsRepository::MASK, $masked['mcpServers']['docs']['headers']['Authorization']);
        $this->assertSame('acme', $masked['mcpServers']['docs']['headers']['X-Tenant']);
        $this->assertSame(VedaSettingsRepository::MASK, $masked['mcpServers']['docs']['env']['API_KEY']);
    }

    public function test_keeps_interpolation_placeholders_visible(): void
    {
        $catalog = new McpCatalog;
        $masked = $catalog->mask([
            'mcpServers' => [
                'docs' => [
                    'url' => 'https://docs.example.test/mcp',
                    'headers' => [
                        'Authorization' => 'Bearer ${env:DOCS_TOKEN}',
                    ],
                ],
            ],
        ]);

        $this->assertSame('Bearer ${env:DOCS_TOKEN}', $masked['mcpServers']['docs']['headers']['Authorization']);
    }

    public function test_loads_env_file_and_skips_comments(): void
    {
        $path = sys_get_temp_dir().'/veda-mcp-env-'.uniqid('', true).'.env';
        file_put_contents($path, <<<'ENV'
# comment
VEDA_A=one
export VEDA_B="two"
VEDA_C='three'
VEDA_D=${env:API_KEY}

ENV);

        try {
            $catalog = new McpCatalog(
                env: fn (string $name): ?string => $name === 'API_KEY' ? 'secret' : null,
            );

            $this->assertSame([
                'VEDA_A' => 'one',
                'VEDA_B' => 'two',
                'VEDA_C' => 'three',
                'VEDA_D' => 'secret',
            ], $catalog->loadEnvFile($path));
        } finally {
            @unlink($path);
        }
    }

    public function test_missing_env_file_is_empty(): void
    {
        $this->assertSame([], (new McpCatalog)->loadEnvFile('/no/such/veda-mcp.env'));
    }

    public function test_normalizes_stdio_server_fields(): void
    {
        $mcp = (new McpCatalog)->normalize([
            'mcpServers' => [
                'local' => [
                    'command' => 'php',
                    'args' => ['server.php'],
                    'env' => ['API_KEY' => 'process-secret'],
                    'envFile' => '${workspaceFolder}/.env',
                    'cwd' => '${workspaceFolder}',
                    'disabled' => true,
                ],
            ],
        ]);

        $this->assertSame([
            'mcpServers' => [
                'local' => [
                    'command' => 'php',
                    'args' => ['server.php'],
                    'env' => ['API_KEY' => 'process-secret'],
                    'envFile' => '${workspaceFolder}/.env',
                    'cwd' => '${workspaceFolder}',
                    'disabled' => true,
                ],
            ],
        ], $mcp);
    }
}
