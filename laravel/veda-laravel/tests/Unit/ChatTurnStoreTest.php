<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Models\VedaChatTurn;
use Veda\Laravel\Services\ChatTurnStore;
use Veda\Laravel\Services\UserMessageComposer;
use Veda\Laravel\Tests\TestCase;

class ChatTurnStoreTest extends TestCase
{
    public function test_save_and_load_frozen_user_turn(): void
    {
        $user = $this->createUser();
        $store = new ChatTurnStore;
        $frozen = (new UserMessageComposer)->compose('Create course', ['LMS surface: course.'], false);

        $store->saveTurn($user, 'chat-abc', 0, $frozen, 'User role: teacher.', 'cache-key-1');

        $loaded = $store->findFrozenUserContent($user, 'chat-abc', 0);
        $this->assertSame($frozen, $loaded);

        $history = $store->applyFrozenTurnsToHistory($user, 'chat-abc', [
            ['role' => 'user', 'content' => 'plain legacy text'],
        ]);

        $this->assertSame($frozen, $history[0]['content']);
    }

    public function test_second_turn_uses_incremented_turn_index(): void
    {
        $user = $this->createUser();
        $store = new ChatTurnStore;
        $first = (new UserMessageComposer)->compose('First', [], false);
        $second = (new UserMessageComposer)->compose('Second', [], false);

        $store->saveTurn($user, 'chat-xyz', 0, $first);
        $store->saveTurn($user, 'chat-xyz', 1, $second);

        $this->assertSame(2, VedaChatTurn::query()->where('chat_id', 'chat-xyz')->count());
        $this->assertSame($second, $store->findFrozenUserContent($user, 'chat-xyz', 1));
        $this->assertSame(1, $store->countUserTurnsInHistory([
            ['role' => 'user', 'content' => $first],
            ['role' => 'assistant', 'content' => 'ok'],
        ]));
    }

    public function test_global_context_restored_when_cache_key_matches(): void
    {
        $user = $this->createUser();
        $store = new ChatTurnStore;
        $store->saveTurn($user, 'chat-g', 0, '<userRequest>hi</userRequest>', 'Stable global block', 'key-match');

        $this->assertSame('Stable global block', $store->findGlobalContext($user, 'chat-g', 'key-match'));
        $this->assertNull($store->findGlobalContext($user, 'chat-g', 'other-key'));
    }

    public function test_upsert_global_context_updates_turn_zero_when_cache_key_changes(): void
    {
        $user = $this->createUser();
        $store = new ChatTurnStore;
        $store->saveTurn($user, 'chat-nav', 0, '<userRequest>x</userRequest>', 'Old global', 'old-key');

        $store->upsertChatGlobalContext($user, 'chat-nav', 'New global', 'new-key');

        $this->assertSame('New global', $store->findGlobalContext($user, 'chat-nav', 'new-key'));
        $this->assertNull($store->findGlobalContext($user, 'chat-nav', 'old-key'));
    }

    public function test_store_disabled_skips_persistence(): void
    {
        config(['veda.chat_turns.enabled' => false]);
        $user = $this->createUser();
        $store = new ChatTurnStore;

        $store->saveTurn($user, 'chat-off', 0, 'frozen');
        $this->assertNull($store->findFrozenUserContent($user, 'chat-off', 0));
    }
}
