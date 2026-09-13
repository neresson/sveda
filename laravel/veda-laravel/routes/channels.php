<?php

use Illuminate\Support\Facades\Broadcast;

$prefix = config('veda.broadcasting.channel_prefix', 'veda');

Broadcast::channel($prefix.'.{userId}.{chatId}', function ($user, $userId, $chatId) {
    $authorize = config('veda.broadcasting.authorize');

    if (is_callable($authorize)) {
        return $authorize($user, $userId, $chatId);
    }

    return (string) $user->getAuthIdentifier() === (string) $userId;
});
