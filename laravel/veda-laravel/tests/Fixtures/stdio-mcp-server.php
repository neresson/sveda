<?php

declare(strict_types=1);

ini_set('display_errors', 'stderr');

while (($line = fgets(STDIN)) !== false) {
    $line = trim($line);
    if ($line === '') {
        continue;
    }

    $message = json_decode($line, true);
    if (! is_array($message)) {
        continue;
    }

    $method = $message['method'] ?? '';
    $id = $message['id'] ?? null;

    if ($id === null) {
        continue;
    }

    $result = match ($method) {
        'initialize' => [
            'protocolVersion' => '2025-11-25',
            'capabilities' => ['tools' => ['listChanged' => false]],
            'serverInfo' => ['name' => 'veda-stdio-fixture', 'version' => '0.1.0'],
        ],
        'tools/list' => [
            'tools' => [[
                'name' => 'echo_env',
                'title' => 'echo_env',
                'description' => 'Returns stdio env values.',
                'inputSchema' => [
                    'type' => 'object',
                    'properties' => (object) [],
                ],
                'annotations' => [],
                '_meta' => [
                    'domain' => 'testing',
                    'mode' => 'read',
                ],
            ]],
        ],
        'tools/call' => [
            'content' => [[
                'type' => 'text',
                'text' => json_encode([
                    'success' => true,
                    'data' => [
                        'token' => (string) getenv('VEDA_STDIO_TOKEN'),
                        'from_file' => (string) getenv('VEDA_STDIO_FROM_FILE'),
                    ],
                ], JSON_UNESCAPED_UNICODE),
            ]],
            'structuredContent' => [
                'success' => true,
                'data' => [
                    'token' => (string) getenv('VEDA_STDIO_TOKEN'),
                    'from_file' => (string) getenv('VEDA_STDIO_FROM_FILE'),
                ],
            ],
            'isError' => false,
        ],
        default => null,
    };

    fwrite(STDOUT, json_encode(
        $result === null
            ? [
                'jsonrpc' => '2.0',
                'id' => $id,
                'error' => ['code' => -32601, 'message' => 'Method not found'],
            ]
            : [
                'jsonrpc' => '2.0',
                'id' => $id,
                'result' => $result,
            ],
        JSON_UNESCAPED_UNICODE,
    )."\n");
    fflush(STDOUT);
}
