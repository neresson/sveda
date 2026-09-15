<?php

namespace Veda\Laravel\Tests\Unit;

use GuzzleHttp\Psr7\Utils;
use Illuminate\Contracts\Events\Dispatcher;
use ReflectionMethod;
use Veda\Laravel\Gateway\VedaResponsesGateway;
use Veda\Laravel\Tests\TestCase;

class DeepSeekStreamParseTest extends TestCase
{
    public function test_stops_parsing_after_response_completed_even_if_body_continues(): void
    {
        $sse = implode("\n", [
            'event: response.created',
            'data: {"type":"response.created","response":{"id":"r1","model":"deepseek-v4-flash"}}',
            '',
            'data: {"type":"response.output_text.delta","delta":"Hello"}',
            '',
            'data: {"type":"response.completed","response":{"id":"r1","status":"completed","usage":{"input_tokens":1,"output_tokens":1}}}',
            '',
            'data: {"type":"response.output_text.delta","delta":"LEAK"}',
            '',
        ]);

        $events = $this->parse($sse);
        $types = array_column($events, 'type');

        $this->assertSame('response.completed', end($types));
        $this->assertSame(['Hello'], array_column(
            array_values(array_filter($events, fn (array $event): bool => ($event['type'] ?? '') === 'response.output_text.delta')),
            'delta',
        ));
    }

    public function test_parses_concatenated_event_and_data_on_one_line(): void
    {
        $sse = 'event: response.output_text.delta data: {"type":"response.output_text.delta","delta":"Hi"}'."\n\n"
            .'data: {"type":"response.completed","response":{"id":"r1","status":"completed"}}'."\n\n";

        $events = $this->parse($sse);

        $this->assertSame('Hi', $events[0]['delta'] ?? null);
        $this->assertSame('response.completed', $events[array_key_last($events)]['type'] ?? null);
    }

    /**
     * @return array<int, array<string, mixed>>
     */
    protected function parse(string $sse): array
    {
        $gateway = new VedaResponsesGateway(app(Dispatcher::class));
        $method = new ReflectionMethod($gateway, 'parseServerSentEvents');

        $events = [];
        foreach ($method->invoke($gateway, Utils::streamFor($sse)) as $event) {
            if (is_array($event)) {
                $events[] = $event;
            }
        }

        return $events;
    }
}
