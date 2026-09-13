<?php

namespace Veda\Laravel\Tests\Fixtures;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Http\Request;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Http\Controllers\VedaStreamController;
use Veda\Laravel\Services\EmbedChatScope;

class CustomStreamController extends VedaStreamController
{
    protected function makeAgent(?Authenticatable $user, ?string $chatId): VedaAgent
    {
        return new CustomVedaAgent($user, $chatId);
    }

    protected function historyScope(Request $request): EmbedChatScope
    {
        return new EmbedChatScope($request->user(), true, 'custom-visitor');
    }
}

class CustomVedaAgent extends VedaAgent {}
