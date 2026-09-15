<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\CodeIndex\CodeSymbolAwareChunkBuilder;
use Veda\Laravel\Tests\TestCase;

class CodeSymbolAwareChunkBuilderTest extends TestCase
{
    public function test_line_chunks_overlap_produces_multiple_spans(): void
    {
        $builder = new CodeSymbolAwareChunkBuilder;
        $lines = [];
        for ($i = 1; $i <= 200; $i++) {
            $lines[] = '// line '.$i.' '.str_repeat('x', 40);
        }
        $content = implode("\n", $lines);

        $chunks = $builder->lineChunks($content, 800, 0.2);

        $this->assertGreaterThan(1, count($chunks));
        $this->assertSame(1, $chunks[0]['start']);
        $this->assertGreaterThan($chunks[0]['start'], $chunks[1]['start']);
    }

    public function test_embedding_text_includes_path_and_symbol(): void
    {
        $builder = new CodeSymbolAwareChunkBuilder;
        $text = $builder->embeddingText('app/Foo.php', 10, 20, 'body', 'Foo::bar', 'method');

        $this->assertStringContainsString('method Foo::bar', $text);
        $this->assertStringContainsString('app/Foo.php', $text);
        $this->assertStringContainsString('body', $text);
    }
}
