<?php

namespace Veda\Laravel\CodeIndex;

use Illuminate\Support\Facades\Log;
use Symfony\Component\Process\Process;
use Veda\Laravel\Models\VedaCodeSource;

class CodeIndexRipgrepService
{
    public function isAvailable(): bool
    {
        $process = new Process(['rg', '--version']);
        $process->setTimeout(5);
        $process->run();

        return $process->isSuccessful();
    }

    /**
     * @param  array<int, string>  $queries
     * @return list<array{path: string, line: int, end_line: int, text: string, rank: int}>
     */
    public function search(VedaCodeSource $source, string $root, array $queries, int $limitPerQuery = 40): array
    {
        if (! $this->isAvailable() || $queries === []) {
            return [];
        }

        $limitPerQuery = max(5, min(80, $limitPerQuery));
        $byKey = [];

        foreach ($queries as $query) {
            $query = trim($query);
            if ($query === '') {
                continue;
            }

            $process = new Process([
                'rg',
                '--json',
                '--max-count', (string) $limitPerQuery,
                '--ignore-case',
                '--line-number',
                '--no-heading',
                $query,
                '.',
            ], $root);
            $process->setTimeout(30);
            $process->run();

            if (! $process->isSuccessful() && $process->getExitCode() !== 1) {
                Log::warning('veda.code_search.rg_failed', [
                    'code_source_id' => $source->id,
                    'stderr' => $process->getErrorOutput(),
                ]);

                continue;
            }

            $rank = 0;
            foreach (explode("\n", trim($process->getOutput())) as $line) {
                if ($line === '') {
                    continue;
                }
                $payload = json_decode($line, true);
                if (! is_array($payload) || ($payload['type'] ?? '') !== 'match') {
                    continue;
                }
                $data = $payload['data'] ?? [];
                $path = isset($data['path']['text']) ? (string) $data['path']['text'] : '';
                if ($path === '') {
                    continue;
                }
                $lineNumber = (int) ($data['line_number'] ?? 1);
                $text = isset($data['lines']['text']) ? (string) $data['lines']['text'] : '';
                $rank++;
                $key = $path."\0".$lineNumber;
                if (! isset($byKey[$key]) || $byKey[$key]['rank'] > $rank) {
                    $byKey[$key] = [
                        'path' => $path,
                        'line' => $lineNumber,
                        'end_line' => $lineNumber,
                        'text' => $text,
                        'rank' => $rank,
                    ];
                }
            }
        }

        return array_values($byKey);
    }
}
