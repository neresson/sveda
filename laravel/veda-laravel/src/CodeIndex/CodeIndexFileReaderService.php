<?php

namespace Veda\Laravel\CodeIndex;

use Veda\Laravel\Models\VedaCodeIndexChunk;
use Veda\Laravel\Models\VedaCodeSource;
use Veda\Laravel\Support\Utf8Text;

class CodeIndexFileReaderService
{
    public function read(
        int $sourceId,
        string $pathRaw,
        int $offset = 0,
        int $length = 0,
        ?int $startLine = null,
        ?int $endLine = null,
    ): array {
        $pathRaw = trim($pathRaw);
        if ($sourceId <= 0 || $pathRaw === '') {
            return ['success' => false, 'error' => 'source_id and path are required'];
        }

        $source = VedaCodeSource::query()->whereKey($sourceId)->first();
        if (! $source) {
            return ['success' => false, 'error' => 'Code source not found'];
        }

        $pathSan = Utf8Text::sanitize($pathRaw, 2048);
        $merged = $this->readFromWorkspace($source, $pathSan);
        if ($merged === null && $pathSan !== $pathRaw) {
            $merged = $this->readFromWorkspace($source, $pathRaw);
        }
        if ($merged === null) {
            $merged = $this->mergeIndexedFileChunks($sourceId, $pathSan);
        }
        if ($merged === null && $pathSan !== $pathRaw) {
            $merged = $this->mergeIndexedFileChunks($sourceId, $pathRaw);
        }
        if ($merged === null) {
            return ['success' => false, 'error' => 'No indexed chunks for this path'];
        }

        $defaultLength = max(1, (int) config('veda.code_index.read_default_chars', 4000));
        $maxLength = max($defaultLength, (int) config('veda.code_index.read_max_chars', 8000));
        $length = $length > 0 ? min($maxLength, $length) : $defaultLength;

        $text = $merged['text'];
        $total = mb_strlen($text, 'UTF-8');
        $totalLines = substr_count($text, "\n") + ($text === '' ? 0 : 1);

        if ($startLine !== null || $endLine !== null) {
            $slice = $this->sliceByLines($text, $startLine, $endLine);
        } else {
            $slice = $this->sliceByOffset($text, max(0, $offset), $length);
        }

        $data = [
            'source_id' => $sourceId,
            'path' => $pathSan !== '' ? $pathSan : $pathRaw,
            'anchor_chunk_id' => $merged['anchor_chunk_id'],
            'content' => $slice['content'],
            'content_total_length' => $total,
            'total_lines' => $totalLines,
            'content_offset' => $slice['content_offset'],
            'content_chunk_length' => $slice['content_chunk_length'],
            'has_more_content' => $slice['has_more_content'],
        ];

        if ($slice['start_line'] !== null && $slice['end_line'] !== null) {
            $data['start_line'] = $slice['start_line'];
            $data['end_line'] = $slice['end_line'];
        }

        if ($slice['has_more_content']) {
            if ($slice['next_offset'] !== null) {
                $data['next_offset'] = $slice['next_offset'];
            }
            if ($slice['next_start_line'] !== null) {
                $data['next_start_line'] = $slice['next_start_line'];
            }
            $data['read_hint'] = $this->buildReadHint($slice);
        }

        return [
            'success' => true,
            'data' => $data,
        ];
    }

    /**
     * @return array{source_id: int, path: string}|null
     */
    public function resolvePathFromChunkId(int $chunkId): ?array
    {
        if ($chunkId <= 0) {
            return null;
        }

        $chunk = VedaCodeIndexChunk::query()
            ->select('code_source_id', 'path')
            ->whereKey($chunkId)
            ->first();

        if (! $chunk) {
            return null;
        }

        return [
            'source_id' => (int) $chunk->code_source_id,
            'path' => (string) $chunk->path,
        ];
    }

    /**
     * @return array{
     *     content: string,
     *     content_offset: int,
     *     content_chunk_length: int,
     *     has_more_content: bool,
     *     next_offset: int|null,
     *     next_start_line: null,
     *     start_line: null,
     *     end_line: null
     * }
     */
    public function sliceByOffset(string $text, int $offset, int $length): array
    {
        $offset = max(0, $offset);
        $length = max(1, $length);
        $total = mb_strlen($text, 'UTF-8');
        $slice = mb_substr($text, $offset, $length, 'UTF-8');
        $sliceLen = mb_strlen($slice, 'UTF-8');
        $hasMore = ($offset + $sliceLen) < $total;

        return [
            'content' => $slice,
            'content_offset' => $offset,
            'content_chunk_length' => $sliceLen,
            'has_more_content' => $hasMore,
            'next_offset' => $hasMore ? $offset + $sliceLen : null,
            'next_start_line' => null,
            'start_line' => null,
            'end_line' => null,
        ];
    }

    /**
     * @return array{
     *     content: string,
     *     content_offset: int,
     *     content_chunk_length: int,
     *     has_more_content: bool,
     *     next_offset: null,
     *     next_start_line: int|null,
     *     start_line: int,
     *     end_line: int
     * }
     */
    public function sliceByLines(string $text, ?int $startLine, ?int $endLine): array
    {
        $lines = preg_split('/\R/u', $text) ?: [''];
        $totalLines = count($lines);
        $maxLines = max(1, (int) config('veda.code_index.read_max_lines', 250));
        $from = max(1, $startLine ?? 1);
        $to = $endLine !== null
            ? min($totalLines, max($from, $endLine))
            : min($totalLines, $from + $maxLines - 1);
        $selected = array_slice($lines, $from - 1, $to - $from + 1);
        $content = implode("\n", $selected);
        $sliceLen = mb_strlen($content, 'UTF-8');
        $hasMore = $to < $totalLines;

        return [
            'content' => $content,
            'content_offset' => 0,
            'content_chunk_length' => $sliceLen,
            'has_more_content' => $hasMore,
            'next_offset' => null,
            'next_start_line' => $hasMore ? $to + 1 : null,
            'start_line' => $from,
            'end_line' => $to,
        ];
    }

    /**
     * @param  array<string, mixed>  $slice
     */
    protected function buildReadHint(array $slice): string
    {
        if ($slice['next_start_line'] !== null) {
            return 'Call again with start_line='.$slice['next_start_line'].' (and optional end_line) to continue line-based reading.';
        }

        if ($slice['next_offset'] !== null) {
            return 'Call again with offset='.$slice['next_offset'].' (or use start_line/end_line for line-based reading).';
        }

        return 'More content is available; continue reading with pagination parameters.';
    }

    /**
     * @return array{text: string, start_line: int, end_line: int, anchor_chunk_id: int|null}|null
     */
    protected function readFromWorkspace(VedaCodeSource $source, string $path): ?array
    {
        $root = app(CodeSourceIndexer::class)->resolveWorkspaceRoot($source);
        if ($root === null) {
            return null;
        }

        $absolute = $root.DIRECTORY_SEPARATOR.str_replace('/', DIRECTORY_SEPARATOR, ltrim($path, '/'));
        if (! is_file($absolute)) {
            return null;
        }

        $content = @file_get_contents($absolute);
        if ($content === false || $content === '') {
            return null;
        }

        $content = Utf8Text::sanitize($content, (int) config('veda.code_index.max_file_bytes', 524288) + 64);
        $lineCount = substr_count($content, "\n") + 1;

        return [
            'text' => $content,
            'start_line' => 1,
            'end_line' => max(1, $lineCount),
            'anchor_chunk_id' => null,
        ];
    }

    /**
     * @return array{text: string, start_line: int, end_line: int, anchor_chunk_id: int}|null
     */
    protected function mergeIndexedFileChunks(int $sourceId, string $path): ?array
    {
        $rows = VedaCodeIndexChunk::query()
            ->where('code_source_id', $sourceId)
            ->where('path', $path)
            ->orderBy('chunk_index')
            ->orderBy('id')
            ->get(['id', 'content', 'start_line', 'end_line']);

        if ($rows->isEmpty()) {
            return null;
        }

        $pieces = [];
        $startLine = null;
        $endLine = null;
        $anchorId = (int) $rows->first()->id;
        foreach ($rows as $row) {
            $pieces[] = (string) $row->content;
            if ($startLine === null) {
                $startLine = (int) $row->start_line;
            }
            $endLine = (int) $row->end_line;
        }
        $text = implode("\n", $pieces);
        $maxMergeChars = (int) config('veda.code_index.max_file_bytes', 524288) + 64;
        if (mb_strlen($text, 'UTF-8') > $maxMergeChars) {
            $text = mb_substr($text, 0, $maxMergeChars, 'UTF-8');
        }

        return [
            'text' => $text,
            'start_line' => $startLine ?? 1,
            'end_line' => $endLine ?? 1,
            'anchor_chunk_id' => $anchorId,
        ];
    }
}
