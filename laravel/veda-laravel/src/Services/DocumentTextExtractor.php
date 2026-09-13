<?php

namespace Veda\Laravel\Services;

use Illuminate\Http\UploadedFile;
use PhpOffice\PhpWord\IOFactory;
use Smalot\PdfParser\Parser;
use Veda\Laravel\VedaManager;

class DocumentTextExtractor
{
    /**
     * @return array{ok: bool, text: string, error?: string}
     */
    public function extract(UploadedFile $file): array
    {
        $custom = app(VedaManager::class)->getDocumentExtractor();
        if ($custom !== null) {
            $result = $custom($file);

            return is_array($result) && isset($result['ok'])
                ? $result
                : ['ok' => false, 'error' => 'Custom document extractor returned an invalid result.'];
        }

        $extension = strtolower((string) $file->getClientOriginalExtension());
        $path = $file->getRealPath();

        if ($path === false || ! is_file($path)) {
            return ['ok' => false, 'error' => 'File is not readable.'];
        }

        if (in_array($extension, ['txt', 'md', 'csv', 'log', 'json'], true)) {
            $text = (string) file_get_contents($path);

            return ['ok' => true, 'text' => $this->normalize($text)];
        }

        if ($extension === 'pdf') {
            return $this->extractPdf($path);
        }

        if (in_array($extension, ['docx', 'xlsx', 'pptx'], true)) {
            return $this->extractOffice($path, $extension);
        }

        return ['ok' => false, 'error' => "Unsupported file type: {$extension}."];
    }

    /**
     * @return array{ok: bool, text: string, error?: string}
     */
    protected function extractPdf(string $path): array
    {
        if (class_exists(Parser::class)) {
            try {
                $parser = new Parser;
                $pdf = $parser->parseFile($path);

                return ['ok' => true, 'text' => $this->normalize($pdf->getText())];
            } catch (\Throwable $e) {
                return ['ok' => false, 'error' => 'PDF parsing failed: '.$e->getMessage()];
            }
        }

        return ['ok' => false, 'error' => 'PDF support requires smalot/pdfparser. Install it or register a custom extractor via Veda::documentExtractor().'];
    }

    /**
     * @return array{ok: bool, text: string, error?: string}
     */
    protected function extractOffice(string $path, string $extension): array
    {
        if (class_exists(IOFactory::class) && $extension === 'docx') {
            try {
                $doc = IOFactory::load($path);
                $text = '';
                foreach ($doc->getSections() as $section) {
                    foreach ($section->getElements() as $element) {
                        if (method_exists($element, 'getText')) {
                            $text .= $element->getText()."\n";
                        }
                    }
                }

                return ['ok' => true, 'text' => $this->normalize($text)];
            } catch (\Throwable $e) {
                return ['ok' => false, 'error' => 'DOCX parsing failed: '.$e->getMessage()];
            }
        }

        return ['ok' => false, 'error' => "Office support ({$extension}) requires phpoffice packages. Install them or register a custom extractor via Veda::documentExtractor()."];
    }

    protected function normalize(string $text): string
    {
        $text = str_replace("\r\n", "\n", $text);
        $text = preg_replace('/[ \t]+/u', ' ', $text) ?? $text;
        $text = preg_replace('/\n{3,}/u', "\n\n", $text) ?? $text;

        return trim($text);
    }
}
