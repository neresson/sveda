<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Veda\Laravel\Services\VedaSettingsRepository;

class VedaEmbedConfigController
{
    public function __invoke(VedaSettingsRepository $settings): JsonResponse
    {
        return response()->json($settings->publicDocument());
    }
}
