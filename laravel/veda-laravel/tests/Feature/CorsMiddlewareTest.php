<?php

namespace Veda\Laravel\Tests\Feature;

use Veda\Laravel\Tests\TestCase;

class CorsMiddlewareTest extends TestCase
{
    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.embed.enabled', true);
        $app['config']->set('veda.cors.allowed_origins', ['http://localhost:8001']);
    }

    public function test_preflight_allows_configured_widget_origin(): void
    {
        $response = $this->call('OPTIONS', '/veda/stream', [], [], [], [
            'HTTP_ORIGIN' => 'http://localhost:8001',
            'HTTP_ACCESS_CONTROL_REQUEST_METHOD' => 'POST',
            'HTTP_ACCESS_CONTROL_REQUEST_HEADERS' => 'content-type,x-veda-embed-token,x-veda-protocol',
        ]);

        $response->assertNoContent();
        $response->assertHeader('Access-Control-Allow-Origin', 'http://localhost:8001');
        $response->assertHeader('Access-Control-Allow-Headers');
        $this->assertStringContainsStringIgnoringCase('x-veda-embed-token', (string) $response->headers->get('Access-Control-Allow-Headers'));
        $this->assertStringContainsStringIgnoringCase('POST', (string) $response->headers->get('Access-Control-Allow-Methods'));
    }

    public function test_preflight_omits_allow_origin_for_unknown_origin(): void
    {
        $response = $this->call('OPTIONS', '/veda/stream', [], [], [], [
            'HTTP_ORIGIN' => 'http://evil.test',
            'HTTP_ACCESS_CONTROL_REQUEST_METHOD' => 'POST',
        ]);

        $response->assertNoContent();
        $this->assertNull($response->headers->get('Access-Control-Allow-Origin'));
    }

    public function test_stream_response_includes_cors_headers_for_widget_origin(): void
    {
        $token = app(\Veda\Laravel\Services\EmbedTokenService::class)->issue('visitor-cors');

        $response = $this->getJson('/veda/chat-histories', [
            'Origin' => 'http://localhost:8001',
            'X-Veda-Embed-Token' => $token,
        ]);

        $response->assertOk();
        $response->assertHeader('Access-Control-Allow-Origin', 'http://localhost:8001');
    }
}
