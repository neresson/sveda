<?php

use Illuminate\Support\Facades\Route;
use Veda\Laravel\Http\Controllers\VedaAdminSettingsController;
use Veda\Laravel\Http\Controllers\VedaAdminUiController;
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
    Route::post('admin/setup', [VedaAdminUiController::class, 'setup'])->name('veda.admin.setup');
    Route::post('admin/login', [VedaAdminUiController::class, 'login'])->name('veda.admin.login');
    Route::post('admin/logout', [VedaAdminUiController::class, 'logout'])->name('veda.admin.logout');
    Route::post('admin/settings', [VedaAdminUiController::class, 'update'])
        ->middleware(VedaAdminSession::class)
        ->name('veda.admin.settings.form');
});
