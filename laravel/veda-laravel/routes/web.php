<?php

use Illuminate\Support\Facades\Route;
use Veda\Laravel\Http\Controllers\VedaChatHistoryController;
use Veda\Laravel\Http\Controllers\VedaDocumentExtractController;
use Veda\Laravel\Http\Controllers\VedaEmbedConfigController;
use Veda\Laravel\Http\Controllers\VedaEmbedTokenController;
use Veda\Laravel\Http\Controllers\VedaStreamController;
use Veda\Laravel\Http\Middleware\VedaAuthorize;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;

Route::group(['middleware' => [VedaEmbedAuth::class, VedaAuthorize::class]], function () {
    Route::post('/stream', [VedaStreamController::class, 'stream'])->name('veda.stream');
    Route::post('/message', [VedaStreamController::class, 'message'])->name('veda.message');

    Route::get('/chat-histories', [VedaChatHistoryController::class, 'index'])->name('veda.chat-histories.index');
    Route::get('/chat-histories/{chatId}', [VedaChatHistoryController::class, 'show'])->name('veda.chat-histories.show');
    Route::patch('/chat-histories/{chatId}', [VedaChatHistoryController::class, 'update'])->name('veda.chat-histories.update');
    Route::delete('/chat-histories/{chatId}', [VedaChatHistoryController::class, 'destroy'])->name('veda.chat-histories.destroy');

    Route::post('/documents/extract', VedaDocumentExtractController::class)->name('veda.documents.extract');
    Route::get('/embed/config', VedaEmbedConfigController::class)->name('veda.embed.config');
});

if (config('veda.embed.enabled', false)) {
    Route::post('/embed/token', VedaEmbedTokenController::class)
        ->middleware(['throttle:'.(string) config('veda.embed.throttle', '10,1')])
        ->name('veda.embed.token');
}
