<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Veda\Laravel\Services\DocumentTextExtractor;

class VedaDocumentExtractController
{
    public function __invoke(Request $request, DocumentTextExtractor $extractor): JsonResponse
    {
        $maxFiles = (int) config('veda.documents.max_files', 5);
        $maxFileKb = (int) config('veda.documents.max_file_kb', 40960);
        $maxTotalChars = (int) config('veda.documents.max_total_chars', 150000);

        $validated = $request->validate([
            'files' => 'required|array|min:1|max:'.$maxFiles,
            'files.*' => [
                'required',
                'file',
                'max:'.$maxFileKb,
                'mimes:pdf,ppt,pptx,pps,ppsx,doc,docx,xls,xlsx,xlsm,ods,txt,csv,md,json,log',
            ],
        ]);

        $items = [];
        $totalChars = 0;

        foreach ($validated['files'] as $file) {
            $filename = (string) $file->getClientOriginalName();

            $result = $extractor->extract($file);

            if (! ($result['ok'] ?? false)) {
                $items[] = [
                    'filename' => $filename,
                    'ok' => false,
                    'error' => $result['error'] ?? 'extract_failed',
                ];

                continue;
            }

            $normalized = trim((string) ($result['text'] ?? ''));

            if ($normalized === '') {
                $items[] = [
                    'filename' => $filename,
                    'ok' => false,
                    'error' => 'empty',
                ];

                continue;
            }

            $remaining = $maxTotalChars - $totalChars;
            $truncated = false;
            if (mb_strlen($normalized) > $remaining) {
                if ($remaining <= 0) {
                    $items[] = [
                        'filename' => $filename,
                        'ok' => false,
                        'error' => 'quota_exceeded',
                    ];

                    continue;
                }
                $normalized = mb_substr($normalized, 0, $remaining);
                $truncated = true;
            }

            $totalChars += mb_strlen($normalized);

            $items[] = [
                'filename' => $filename,
                'ok' => true,
                'text' => $normalized,
                'truncated' => $truncated,
            ];
        }

        return response()->json([
            'items' => $items,
        ]);
    }
}
