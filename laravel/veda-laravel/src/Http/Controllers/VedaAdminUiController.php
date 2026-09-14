<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\View\View;
use Veda\Laravel\Http\Requests\UpdateAdminSettingsRequest;
use Veda\Laravel\Services\VedaAdminAccess;
use Veda\Laravel\Services\VedaSettingsRepository;

class VedaAdminUiController
{
    public function show(Request $request, VedaSettingsRepository $settings, VedaAdminAccess $access): View
    {
        if (! $access->isConfigured()) {
            return view('veda::admin.setup');
        }

        if ($request->session()->get('veda.admin') !== true) {
            return view('veda::admin.login');
        }

        return view('veda::admin.settings', [
            'settings' => $settings->maskedDocument(),
        ]);
    }

    public function setup(Request $request, VedaAdminAccess $access): RedirectResponse
    {
        if ($access->isConfigured()) {
            return redirect()->route('veda.admin');
        }

        $validated = $request->validate([
            'key' => ['required', 'string', 'min:16', 'confirmed'],
        ], [
            'key.required' => 'required',
            'key.min' => 'min',
            'key.confirmed' => 'confirmed',
        ]);

        $access->store($validated['key']);
        $request->session()->put('veda.admin', true);

        return redirect()->route('veda.admin');
    }

    public function login(Request $request, VedaAdminAccess $access): RedirectResponse
    {
        if (! $access->isConfigured()) {
            return redirect()->route('veda.admin');
        }

        $key = trim((string) $request->input('key', ''));
        if (! $access->matches($key)) {
            return back()->withErrors(['key' => 'invalid']);
        }

        $request->session()->put('veda.admin', true);

        return redirect()->route('veda.admin');
    }

    public function logout(Request $request): RedirectResponse
    {
        $request->session()->forget('veda.admin');

        return redirect()->route('veda.admin');
    }

    public function update(UpdateAdminSettingsRequest $request, VedaSettingsRepository $settings): JsonResponse|RedirectResponse
    {
        $document = $settings->update($request->validated());

        if ($request->wantsJson()) {
            return response()->json($document);
        }

        return redirect()->route('veda.admin');
    }
}
