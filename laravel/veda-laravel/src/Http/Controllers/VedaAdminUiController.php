<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\View\View;
use Veda\Laravel\Http\Requests\UpdateAdminSettingsRequest;
use Veda\Laravel\Services\VedaAdminAccess;
use Veda\Laravel\Services\VedaAdminDashboard;
use Veda\Laravel\Services\VedaAdminUsage;
use Veda\Laravel\Services\VedaAppearance;
use Veda\Laravel\Services\VedaModelCatalog;
use Veda\Laravel\Services\VedaSettingsRepository;

class VedaAdminUiController
{
    public const PAGES = ['dashboard', 'usage', 'runtime', 'models', 'mcp', 'prompts', 'appearance', 'sources'];

    public const SECTIONS = ['usage', 'runtime', 'models', 'mcp', 'prompts', 'appearance', 'sources'];

    public function show(
        Request $request,
        VedaSettingsRepository $settings,
        VedaAdminAccess $access,
        VedaAdminDashboard $dashboard,
        VedaAdminUsage $usage,
        VedaModelCatalog $catalog,
    ): View|JsonResponse {
        if (! $access->isConfigured()) {
            if ($request->expectsJson()) {
                return response()->json(['message' => 'Unauthenticated.'], 401);
            }

            return view('veda::admin.setup');
        }

        if ($request->session()->get('veda.admin') !== true) {
            if ($request->expectsJson()) {
                return response()->json(['message' => 'Unauthenticated.'], 401);
            }

            return view('veda::admin.login');
        }

        $page = $this->adminPage($request);
        $viewData = [
            'settings' => $settings->maskedDocument(),
            'page' => $page,
            'stats' => $page === 'dashboard' ? $dashboard->snapshot() : null,
            'usage' => $page === 'usage' ? $usage->snapshot($request) : null,
            'chat' => $this->chatPayload($catalog),
        ];
        $payload = $this->clientPayload($viewData);

        if ($request->expectsJson()) {
            return response()->json($payload);
        }

        return view('veda::admin.settings', [
            ...$viewData,
            'payload' => $payload,
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

    private function adminPage(Request $request): string
    {
        $page = (string) $request->route('page', 'dashboard');

        return in_array($page, self::PAGES, true) ? $page : 'dashboard';
    }

    /**
     * @param  array{settings: mixed, page: string, stats: mixed, usage: mixed, chat: array<string, mixed>}  $viewData
     * @return array<string, mixed>
     */
    private function clientPayload(array $viewData): array
    {
        return [
            'page' => $viewData['page'],
            'csrf' => csrf_token(),
            'saveUrl' => route('veda.admin.settings.form'),
            'logoutUrl' => route('veda.admin.logout'),
            'urls' => [
                'dashboard' => route('veda.admin'),
                'usage' => route('veda.admin.section', ['page' => 'usage']),
                'runtime' => route('veda.admin.section', ['page' => 'runtime']),
                'models' => route('veda.admin.section', ['page' => 'models']),
                'mcp' => route('veda.admin.section', ['page' => 'mcp']),
                'prompts' => route('veda.admin.section', ['page' => 'prompts']),
                'appearance' => route('veda.admin.section', ['page' => 'appearance']),
                'sources' => route('veda.admin.section', ['page' => 'sources']),
            ],
            'codeIndex' => [
                'sources' => route('veda.admin.code-index.sources'),
                'progress' => route('veda.admin.code-index.progress'),
                'store' => route('veda.admin.code-index.store'),
                'sourceBase' => url('/'.trim((string) config('veda.prefix', 'veda'), '/').'/admin/code-index/sources'),
                'localBrowse' => route('veda.admin.code-index.local-browse'),
                'localPreview' => route('veda.admin.code-index.local-preview'),
                'estimate' => route('veda.admin.code-index.estimate'),
            ],
            'appearancePresets' => VedaAppearance::presets(),
            'settings' => $viewData['settings'],
            'stats' => $viewData['stats'],
            'usage' => $viewData['usage'],
            'chat' => $viewData['chat'],
        ];
    }

    /**
     * @return array{sessionUrl: string, prefix: string, protocol: string, models: array<int, array<string, mixed>>}
     */
    private function chatPayload(VedaModelCatalog $catalog): array
    {
        return [
            'sessionUrl' => route('veda.admin.chat-session'),
            'prefix' => trim((string) config('veda.prefix', 'veda'), '/'),
            'protocol' => (string) config('veda.protocol', 'veda'),
            'models' => array_values($catalog->publicModels()),
        ];
    }
}
