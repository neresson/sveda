<?php

namespace Veda\Laravel\Tests\Unit\Gateway\Yandex;

use Veda\Laravel\Gateway\Yandex\YandexResponsesStreamAccumulator;
use Veda\Laravel\Tests\TestCase;

class YandexResponsesStreamAccumulatorTest extends TestCase
{
    public function test_parses_completed_response_with_text_content_parts(): void
    {
        $accumulator = new YandexResponsesStreamAccumulator;
        $accumulator->applyCompletedResponse([
            'status' => 'completed',
            'output_text' => '',
            'output' => [
                [
                    'type' => 'message',
                    'role' => 'assistant',
                    'content' => [
                        ['type' => 'text', 'text' => 'Это страница задач CRM.'],
                    ],
                ],
            ],
            'usage' => [
                'input_tokens' => 100,
                'output_tokens' => 20,
                'total_tokens' => 120,
            ],
        ]);

        $result = $accumulator->toStreamResult();

        $this->assertSame('Это страница задач CRM.', $result['content']);
        $this->assertSame(120, $result['tokens_used']);
    }

    public function test_parses_output_text_delta_events(): void
    {
        $accumulator = new YandexResponsesStreamAccumulator;
        $accumulator->applyEvent('response.output_text.delta', [
            'type' => 'response.output_text.delta',
            'delta' => 'Привет',
        ]);
        $accumulator->applyEvent('response.output_text.delta', [
            'type' => 'response.output_text.delta',
            'delta' => ', мир',
        ]);

        $this->assertSame('Привет, мир', $accumulator->content());
    }

    public function test_accumulated_text_deltas_do_not_repeat_full_content(): void
    {
        $text = 'Вот подсвеченные элементы в шапке.';

        $accumulator = new YandexResponsesStreamAccumulator;
        $accumulator->applyEvent('response.output_text.delta', [
            'type' => 'response.output_text.delta',
            'delta' => $text,
        ]);
        $accumulator->applyEvent('response.output_text.delta', [
            'type' => 'response.output_text.delta',
            'delta' => $text,
        ]);
        $accumulator->applyEvent('response.output_text.delta', [
            'type' => 'response.output_text.delta',
            'delta' => $text,
        ]);

        $this->assertSame($text, $accumulator->content());
    }

    public function test_completed_response_does_not_duplicate_streamed_text(): void
    {
        $text = "Вот подсвеченные элементы\n\n```scorpiogpt-host\n{\"highlights\":[]}\n```";

        $accumulator = new YandexResponsesStreamAccumulator;
        $accumulator->applyEvent('response.output_text.delta', [
            'type' => 'response.output_text.delta',
            'delta' => $text,
        ]);
        $accumulator->applyEvent('response.output_item.done', [
            'type' => 'response.output_item.done',
            'item' => [
                'type' => 'message',
                'role' => 'assistant',
                'content' => [
                    ['type' => 'text', 'text' => $text],
                ],
            ],
        ]);
        $accumulator->applyCompletedResponse([
            'status' => 'completed',
            'output_text' => $text,
            'output' => [
                [
                    'type' => 'message',
                    'role' => 'assistant',
                    'content' => [
                        ['type' => 'text', 'text' => $text],
                    ],
                ],
            ],
        ]);

        $this->assertSame($text, $accumulator->content());
    }

    public function test_function_call_argument_deltas_do_not_leak_into_assistant_content(): void
    {
        $accumulator = new YandexResponsesStreamAccumulator;
        $accumulator->applyEvent('response.function_call_arguments.delta', [
            'type' => 'response.function_call_arguments.delta',
            'call_id' => 'call-1',
            'name' => 'get_host_page_outline',
            'delta' => '{"query":"Должник"}',
        ]);
        $accumulator->applyEvent('response.output_text.delta', [
            'type' => 'response.output_text.delta',
            'delta' => 'Вот три поля в шапке:',
        ]);

        $result = $accumulator->toStreamResult();

        $this->assertSame('Вот три поля в шапке:', $result['content']);
        $this->assertCount(1, $result['tool_calls']);
        $this->assertSame('{"query":"Должник"}', $result['tool_calls'][0]['function']['arguments']);
    }

    public function test_parses_single_json_stream_body(): void
    {
        $accumulator = new YandexResponsesStreamAccumulator;
        $accumulator->applyRawStreamBody(json_encode([
            'status' => 'completed',
            'output_text' => 'Ответ из одного JSON-блока',
            'usage' => ['total_tokens' => 10],
        ], JSON_UNESCAPED_UNICODE));

        $this->assertSame('Ответ из одного JSON-блока', $accumulator->content());
    }
}
