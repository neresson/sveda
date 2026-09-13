<?php

namespace Veda\Laravel\Services;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Support\Collection;
use Veda\Laravel\Models\VedaChatTurn;

class ChatTurnStore
{
    public function isEnabled(): bool
    {
        return (bool) config('veda.chat_turns.enabled', true);
    }

    /**
     * @param  array<int, array<string, mixed>>  $history
     */
    public function countUserTurnsInHistory(array $history): int
    {
        $count = 0;
        foreach ($history as $message) {
            if (($message['role'] ?? '') === 'user') {
                $count++;
            }
        }

        return $count;
    }

    /**
     * @return Collection<int, VedaChatTurn>
     */
    public function loadTurns(Authenticatable $user, string $chatId): Collection
    {
        return VedaChatTurn::query()
            ->where('user_id', $user->getAuthIdentifier())
            ->where('chat_id', $chatId)
            ->orderBy('turn_index')
            ->get()
            ->keyBy('turn_index');
    }

    /**
     * @param  array<int, array<string, mixed>>  $history
     * @return array<int, array<string, mixed>>
     */
    public function applyFrozenTurnsToHistory(Authenticatable $user, string $chatId, array $history): array
    {
        if (! $this->isEnabled() || $history === []) {
            return $history;
        }

        $turns = $this->loadTurns($user, $chatId);
        if ($turns->isEmpty()) {
            return $history;
        }

        $userTurnIndex = 0;
        foreach ($history as $index => $message) {
            if (($message['role'] ?? '') !== 'user') {
                continue;
            }
            $stored = $turns->get($userTurnIndex);
            if ($stored instanceof VedaChatTurn && $stored->frozen_user_content !== '') {
                $history[$index]['content'] = $stored->frozen_user_content;
            }
            $userTurnIndex++;
        }

        return $history;
    }

    public function findFrozenUserContent(Authenticatable $user, string $chatId, int $turnIndex): ?string
    {
        if (! $this->isEnabled()) {
            return null;
        }

        $turn = VedaChatTurn::query()
            ->where('user_id', $user->getAuthIdentifier())
            ->where('chat_id', $chatId)
            ->where('turn_index', $turnIndex)
            ->first();

        if (! $turn instanceof VedaChatTurn) {
            return null;
        }

        $content = trim($turn->frozen_user_content);

        return $content !== '' ? $turn->frozen_user_content : null;
    }

    public function findGlobalContext(Authenticatable $user, string $chatId, string $currentCacheKey): ?string
    {
        if (! $this->isEnabled()) {
            return null;
        }

        $turn = VedaChatTurn::query()
            ->where('user_id', $user->getAuthIdentifier())
            ->where('chat_id', $chatId)
            ->where('turn_index', 0)
            ->first();

        if (! $turn instanceof VedaChatTurn) {
            return null;
        }

        if ($turn->global_context_cache_key !== $currentCacheKey) {
            return null;
        }

        $rendered = trim((string) ($turn->global_context_rendered ?? ''));

        return $rendered !== '' ? $turn->global_context_rendered : null;
    }

    public function upsertChatGlobalContext(
        Authenticatable $user,
        string $chatId,
        string $stableGlobalContext,
        string $globalContextCacheKey,
    ): void {
        if (! $this->isEnabled() || $stableGlobalContext === '') {
            return;
        }

        $existing = VedaChatTurn::query()
            ->where('user_id', $user->getAuthIdentifier())
            ->where('chat_id', $chatId)
            ->where('turn_index', 0)
            ->first();

        if (! $existing instanceof VedaChatTurn) {
            return;
        }

        $existing->update([
            'global_context_rendered' => $stableGlobalContext,
            'global_context_cache_key' => $globalContextCacheKey,
        ]);
    }

    public function saveTurn(
        Authenticatable $user,
        string $chatId,
        int $turnIndex,
        string $frozenUserContent,
        ?string $globalContextRendered = null,
        ?string $globalContextCacheKey = null,
    ): VedaChatTurn {
        $attributes = [
            'frozen_user_content' => $frozenUserContent,
        ];

        if ($turnIndex === 0) {
            $attributes['global_context_rendered'] = $globalContextRendered;
            $attributes['global_context_cache_key'] = $globalContextCacheKey;
        }

        return VedaChatTurn::query()->updateOrCreate(
            [
                'user_id' => $user->getAuthIdentifier(),
                'chat_id' => $chatId,
                'turn_index' => $turnIndex,
            ],
            $attributes
        );
    }
}
