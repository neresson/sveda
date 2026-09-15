<?php

namespace Veda\Laravel\CodeIndex;

use Veda\Laravel\Models\VedaCodeSource;

class CodeIndexSourcesContext
{
    protected const MAX_DESCRIPTION_CHARS = 600;

    protected const MAX_TOTAL_CHARS = 8000;

    public function build(): string
    {
        $sources = VedaCodeSource::query()
            ->where('status', 'ready')
            ->orderBy('name')
            ->get(['id', 'name', 'description', 'provider']);

        if ($sources->isEmpty()) {
            return '';
        }

        $lines = [];
        foreach ($sources as $source) {
            $name = trim((string) $source->name);
            if ($name === '') {
                $name = 'source #'.$source->id;
            }
            $desc = trim((string) ($source->description ?? ''));
            if (mb_strlen($desc) > self::MAX_DESCRIPTION_CHARS) {
                $desc = mb_substr($desc, 0, self::MAX_DESCRIPTION_CHARS - 1).'…';
            }
            $provider = trim((string) $source->provider);
            $line = '- source_id '.$source->id.': "'.$name.'" ('.$provider.')';
            if ($desc !== '') {
                $line .= '. Administrator context: '.$desc;
            }
            $lines[] = $line;
        }

        $out = implode("\n", $lines);
        if (mb_strlen($out) > self::MAX_TOTAL_CHARS) {
            $out = mb_substr($out, 0, self::MAX_TOTAL_CHARS - 1).'…';
        }

        return $out;
    }

    public function promptSection(): string
    {
        $block = $this->build();
        if ($block === '') {
            return '';
        }

        $sources = $this->renderTemplate('code_index_sources.txt', ['sources_block' => $block]);
        $research = $this->renderTemplate('code_index_research.txt');

        return trim(implode("\n\n", array_filter([$sources, $research], fn ($part) => trim($part) !== '')));
    }

    /**
     * @param  array<string, string>  $replacements
     */
    protected function renderTemplate(string $template, array $replacements = []): string
    {
        $configured = config('veda.prompts.path');
        $base = is_string($configured) && $configured !== ''
            ? $configured
            : dirname(__DIR__, 2).'/resources/prompts/system';
        $path = rtrim($base, '/').'/'.$template;
        $content = is_file($path) ? (string) file_get_contents($path) : '';
        if ($content === '') {
            return '';
        }

        foreach ($replacements as $key => $value) {
            $content = str_replace('{{'.$key.'}}', (string) $value, $content);
        }

        return trim($content);
    }
}
