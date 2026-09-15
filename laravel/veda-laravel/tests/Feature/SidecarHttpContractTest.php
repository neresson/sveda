<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Http\UploadedFile;
use Illuminate\Support\Facades\Queue;
use Laravel\Ai\Ai;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Services\HostMcpToolGateway;
use Veda\Laravel\Streaming\VedaWireProtocolMapper;
use Veda\Laravel\Tests\TestCase;

class SidecarHttpContractTest extends TestCase
{
    /**
     * @return array<string, mixed>
     */
    protected function sidecarContract(): array
    {
        $path = dirname(__DIR__, 2).'/../../packages/protocol/contracts/sidecar.v1.json';
        $this->assertFileExists($path);

        $decoded = json_decode((string) file_get_contents($path), true);
        $this->assertIsArray($decoded);

        return $decoded;
    }

    protected function defineEnvironment($app): void
    {
        parent::defineEnvironment($app);

        $app['config']->set('veda.embed.enabled', true);
        $app['config']->set('veda.title_generation.enabled', false);
        $app['config']->set('veda.compaction.enabled', false);
        $app['config']->set('queue.default', 'sync');
    }

    public function test_contract_version_matches_wire_mapper(): void
    {
        $contract = $this->sidecarContract();

        $this->assertSame(VedaWireProtocolMapper::VERSION, $contract['version']);
        $this->assertSame(VedaWireProtocolMapper::VERSION, $contract['headers']['streamResponse']['X-Veda-Protocol-Version']);
    }

    public function test_cors_and_mcp_headers_match_contract(): void
    {
        $contract = $this->sidecarContract();

        $configured = array_values(array_map('strval', (array) config('veda.cors.allowed_headers')));
        sort($configured);
        $expected = array_values(array_map('strval', $contract['cors']['allowedHeaders']));
        sort($expected);
        $this->assertSame($expected, $configured);

        $this->assertSame('X-Veda-Page-Context', HostMcpToolGateway::PAGE_CONTEXT_HEADER);
        $this->assertSame('X-Veda-Chat-Id', HostMcpToolGateway::CHAT_ID_HEADER);
        $this->assertContains(HostMcpToolGateway::PAGE_CONTEXT_HEADER, $contract['headers']['mcpOutbound']);
        $this->assertContains(HostMcpToolGateway::CHAT_ID_HEADER, $contract['headers']['mcpOutbound']);
    }

    public function test_embed_token_response_matches_contract(): void
    {
        $contract = $this->sidecarContract();

        $response = $this->postJson('/veda/embed/token', [
            'visitor_id' => 'visitor-contract',
        ]);

        $response->assertOk();
        $response->assertJsonStructure($contract['embedToken']['responseRequired']);
        $this->assertSame('visitor-contract', $response->json('visitor_id'));
        $this->assertGreaterThanOrEqual(60, (int) $response->json('expires_in'));
        $this->assertNotSame('', (string) $response->json('token'));
    }

    public function test_stream_headers_and_events_match_contract(): void
    {
        Queue::fake();
        $contract = $this->sidecarContract();
        $user = $this->createUser();
        Ai::fakeAgent(VedaAgent::class, ['Hello contract']);

        $response = $this->actingAs($user)->postJson('/veda/stream', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-contract-stream',
        ], [
            'Accept' => $contract['accept']['vedaStream'],
        ]);

        $response->assertOk();
        $this->assertStringStartsWith($contract['headers']['streamResponse']['Content-Type'], (string) $response->headers->get('Content-Type'));
        $response->assertHeader('X-Veda-Protocol-Version', $contract['headers']['streamResponse']['X-Veda-Protocol-Version']);
        $this->assertStringContainsString(
            $contract['headers']['streamResponse']['Cache-Control'],
            (string) $response->headers->get('Cache-Control'),
        );
        $response->assertHeader('X-Accel-Buffering', $contract['headers']['streamResponse']['X-Accel-Buffering']);

        $content = $response->streamedContent();
        $this->assertStringContainsString($contract['sseDoneLine'], $content);

        $types = [];
        foreach (preg_split("/\r\n|\n|\r/", $content) ?: [] as $line) {
            if (! str_starts_with($line, 'data: ') || trim($line) === $contract['sseDoneLine']) {
                continue;
            }

            $payload = json_decode(substr($line, 6), true);
            if (is_array($payload) && isset($payload['type']) && is_string($payload['type'])) {
                $types[] = $payload['type'];
            }
        }

        $this->assertContains('message.start', $types);
        $this->assertContains('text.delta', $types);
        $this->assertContains('message.end', $types);
        $this->assertSame([], array_values(array_diff(array_unique($types), $contract['streamEvents'])));
    }

    public function test_message_and_histories_match_contract(): void
    {
        Queue::fake();
        $contract = $this->sidecarContract();
        $user = $this->createUser();
        Ai::fakeAgent(VedaAgent::class, ['Plain contract']);

        $message = $this->actingAs($user)->postJson('/veda/message', [
            'messages' => [
                ['id' => 'm1', 'role' => 'user', 'content' => 'Say hello'],
            ],
            'chatId' => 'chat-contract-message',
        ]);

        $message->assertOk();
        $message->assertJsonStructure($contract['message']['responseRequired']);
        $this->assertSame('Plain contract', $message->json('explanation'));

        $list = $this->actingAs($user)->getJson('/veda/chat-histories');
        $list->assertOk();
        $list->assertJsonStructure([
            $contract['histories']['listKey'] => [
                '*' => $contract['histories']['summaryRequired'],
            ],
        ]);

        $detail = $this->actingAs($user)->getJson('/veda/chat-histories/chat-contract-message');
        $detail->assertOk();
        $detail->assertJsonStructure([
            $contract['histories']['detailKey'] => $contract['histories']['detailRequired'],
        ]);

        $rename = $this->actingAs($user)->patchJson('/veda/chat-histories/chat-contract-message', [
            'title' => 'Contract title',
        ]);
        $rename->assertOk();
        $rename->assertJsonStructure($contract['histories']['mutationResponseRequired']);

        $delete = $this->actingAs($user)->deleteJson('/veda/chat-histories/chat-contract-message');
        $delete->assertOk();
        $delete->assertJsonStructure($contract['histories']['mutationResponseRequired']);
    }

    public function test_document_extract_matches_contract(): void
    {
        $contract = $this->sidecarContract();
        $user = $this->createUser();
        $file = UploadedFile::fake()->createWithContent('note.txt', "hello from contract\n");

        $response = $this->actingAs($user)->post('/veda/documents/extract', [
            $contract['documentsExtract']['requestField'] => [$file],
        ]);

        $response->assertOk();
        $response->assertJsonStructure([
            $contract['documentsExtract']['responseKey'] => [
                '*' => $contract['documentsExtract']['itemRequired'],
            ],
        ]);
        $this->assertTrue((bool) $response->json('items.0.ok'));
        $this->assertSame('note.txt', $response->json('items.0.filename'));
    }
}
