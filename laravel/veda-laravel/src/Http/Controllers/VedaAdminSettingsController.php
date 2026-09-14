<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Veda\Laravel\Http\Requests\UpdateAdminSettingsRequest;
use Veda\Laravel\Services\VedaSettingsRepository;

class VedaAdminSettingsController
{
    public function show(VedaSettingsRepository $settings): JsonResponse
    {
        return response()->json($settings->maskedDocument());
    }

    public function update(UpdateAdminSettingsRequest $request, VedaSettingsRepository $settings): JsonResponse
    {
        return response()->json($settings->update($request->validated()));
    }
}
