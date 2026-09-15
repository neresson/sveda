<?php

namespace Veda\Laravel\CodeIndex;

class CodeSymbolAwareChunkBuilder
{
    public const CHUNK_SCHEMA_VERSION = 2;

    public const KIND_LINES = 'lines';

    public const KIND_SYMBOL = 'symbol';

    /**
     * @return list<array{start:int,end:int,text:string}>
     */
    public function lineChunks(string $content, ?int $maxChars = null, ?float $overlapRatio = null): array
    {
        $maxChars = $maxChars ?? (int) config('veda.code_index.chunk_max_chars', 1600);
        $maxChars = max(400, min(2000, $maxChars));
        $overlapRatio = $overlapRatio ?? (float) config('veda.code_index.chunk_overlap_ratio', 0.18);
        $overlapRatio = max(0.05, min(0.35, $overlapRatio));
        $overlapChars = (int) floor($maxChars * $overlapRatio);

        $lines = preg_split('/\R/', $content);
        if (! is_array($lines) || $lines === []) {
            return [['start' => 1, 'end' => 1, 'text' => mb_substr($content, 0, $maxChars, 'UTF-8')]];
        }

        $chunks = [];
        $buf = '';
        $startLine = 1;
        $lineNo = 0;

        foreach ($lines as $line) {
            $lineNo++;
            $candidate = $buf === '' ? $line : $buf."\n".$line;
            if (strlen($candidate) > $maxChars && $buf !== '') {
                $chunks[] = [
                    'start' => $startLine,
                    'end' => $lineNo - 1,
                    'text' => $buf,
                ];
                $buf = $this->tailWithOverlap($buf, $overlapChars).($overlapChars > 0 && $buf !== '' ? "\n" : '').$line;
                $startLine = max(1, $lineNo - $this->countLines($buf) + 1);
            } else {
                $buf = $candidate;
            }
        }

        if ($buf !== '') {
            $chunks[] = [
                'start' => $startLine,
                'end' => $lineNo,
                'text' => $buf,
            ];
        }

        return $chunks === [] ? [['start' => 1, 'end' => 1, 'text' => '']] : $chunks;
    }

    public function contentFingerprint(string $path, int $startLine, int $endLine, string $content, string $kind = self::KIND_LINES): string
    {
        return hash('sha256', $kind."\0".$path."\0".$startLine."\0".$endLine."\0".$content);
    }

    public function embeddingText(
        string $path,
        int $startLine,
        int $endLine,
        string $content,
        ?string $qualifiedName = null,
        ?string $symbolKind = null,
    ): string {
        $pathDisplay = mb_strlen($path) > 448 ? mb_substr($path, 0, 447, 'UTF-8').'…' : $path;
        $header = [];
        if ($qualifiedName !== null && $qualifiedName !== '') {
            $header[] = trim(($symbolKind ?? 'symbol').' '.$qualifiedName);
        }
        $header[] = 'File: '.$pathDisplay.' lines '.$startLine.'-'.$endLine;

        return implode("\n", $header)."\n".$content;
    }

    protected function tailWithOverlap(string $text, int $overlapChars): string
    {
        if ($overlapChars <= 0 || $text === '') {
            return '';
        }

        if (strlen($text) <= $overlapChars) {
            return $text;
        }

        return substr($text, -$overlapChars);
    }

    protected function countLines(string $text): int
    {
        if ($text === '') {
            return 0;
        }

        return substr_count($text, "\n") + 1;
    }
}
