<?php

use Illuminate\Support\Facades\Route;
use Veda\Laravel\Http\Controllers\VedaAdminChatSessionController;
use Veda\Laravel\Http\Controllers\VedaAdminSettingsController;
use Veda\Laravel\Http\Controllers\VedaAdminUiController;
use Veda\Laravel\Http\Controllers\VedaCodeIndexController;
use Veda\Laravel\Http\Middleware\VedaAdminAuth;
use Veda\Laravel\Http\Middleware\VedaAdminSession;

Route::get('admin/settings', [VedaAdminSettingsController::class, 'show'])
    ->middleware(VedaAdminAuth::class)
    ->name('veda.admin.settings.show');

Route::put('admin/settings', [VedaAdminSettingsController::class, 'update'])
    ->middleware(VedaAdminAuth::class)
    ->name('veda.admin.settings.update');

Route::middleware('web')->group(function () {
    Route::get('admin', [VedaAdminUiController::class, 'show'])->name('veda.admin');
    Route::get('admin/{page}', [VedaAdminUiController::class, 'show'])
        ->whereIn('page', VedaAdminUiController::SECTIONS)
        ->name('veda.admin.section');
    Route::post('admin/setup', [VedaAdminUiController::class, 'setup'])->name('veda.admin.setup');
    Route::post('admin/login', [VedaAdminUiController::class, 'login'])->name('veda.admin.login');
    Route::post('admin/logout', [VedaAdminUiController::class, 'logout'])->name('veda.admin.logout');
    Route::post('admin/settings', [VedaAdminUiController::class, 'update'])
        ->middleware(VedaAdminSession::class)
        ->name('veda.admin.settings.form');
    Route::post('admin/chat-session', VedaAdminChatSessionController::class)
        ->middleware(VedaAdminSession::class)
        ->name('veda.admin.chat-session');

    Route::middleware(VedaAdminSession::class)->prefix('admin/code-index')->group(function () {
        Route::get('sources', [VedaCodeIndexController::class, 'index'])->name('veda.admin.code-index.sources');
        Route::get('sources-progress', [VedaCodeIndexController::class, 'progress'])->name('veda.admin.code-index.progress');
        Route::post('sources', [VedaCodeIndexController::class, 'store'])->name('veda.admin.code-index.store');
        Route::patch('sources/{codeSource}', [VedaCodeIndexController::class, 'update'])->name('veda.admin.code-index.update');
        Route::delete('sources/{codeSource}', [VedaCodeIndexController::class, 'destroy'])->name('veda.admin.code-index.destroy');
        Route::post('sources/{codeSource}/cancel-indexing', [VedaCodeIndexController::class, 'cancel'])->name('veda.admin.code-index.cancel');
        Route::post('sources/{codeSource}/reindex', [VedaCodeIndexController::class, 'reindex'])->name('veda.admin.code-index.reindex');
        Route::get('sources/{codeSource}/scope', [VedaCodeIndexController::class, 'scope'])->name('veda.admin.code-index.scope');
        Route::post('sources/{codeSource}/estimate-footprint', [VedaCodeIndexController::class, 'estimateSourceFootprint'])->name('veda.admin.code-index.source-estimate');
        Route::get('local-browse', [VedaCodeIndexController::class, 'browse'])->name('veda.admin.code-index.local-browse');
        Route::post('local-preview', [VedaCodeIndexController::class, 'preview'])->name('veda.admin.code-index.local-preview');
        Route::post('estimate-footprint', [VedaCodeIndexController::class, 'estimateFootprint'])->name('veda.admin.code-index.estimate');
    });
});
