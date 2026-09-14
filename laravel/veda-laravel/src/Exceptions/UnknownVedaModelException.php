<?php

namespace Veda\Laravel\Exceptions;

use Illuminate\Http\JsonResponse;
use RuntimeException;

class UnknownVedaModelException extends RuntimeException
{
    public function __construct(string $modelId = '')
    {
        $suffix = $modelId !== '' ? " [{$modelId}]" : '';

        parent::__construct('Unknown Veda model'.$suffix.'.');
    }

    public function render(): JsonResponse
    {
        return response()->json([
            'message' => $this->getMessage(),
        ], 422);
    }
}
