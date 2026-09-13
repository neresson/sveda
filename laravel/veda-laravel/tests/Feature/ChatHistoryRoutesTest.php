<?php

namespace Veda\Laravel\Tests\Feature;

use Veda\Laravel\Models\VedaChatHistory;
use Veda\Laravel\Tests\TestCase;

class ChatHistoryRoutesTest extends TestCase
{
    public function test_history_routes_require_authentication(): void
    {
        $this->getJson('/veda/chat-histories')->assertStatus(401);
    }

    public function test_index_returns_user_histories(): void
    {
        $user = $this->createUser();

        VedaChatHistory::query()->create([
            'user_id' => $user->getAuthIdentifier(),
            'visitor_id' => '',
            'chat_id' => 'chat-1',
            'title' => 'First chat',
            'messages' => [],
            'conversation_history' => [],
            'tokens_used' => 0,
            'version' => 1,
        ]);

        $response = $this->actingAs($user)->getJson('/veda/chat-histories');

        $response->assertOk();
        $response->assertJsonCount(1, 'histories');
        $this->assertSame('chat-1', $response->json('histories.0.id'));
        $this->assertSame('chat-1', $response->json('histories.0.chatId'));
    }

    public function test_show_returns_single_history(): void
    {
        $user = $this->createUser();

        VedaChatHistory::query()->create([
            'user_id' => $user->getAuthIdentifier(),
            'visitor_id' => '',
            'chat_id' => 'chat-42',
            'title' => 'Answered chat',
            'messages' => [['role' => 'user', 'content' => 'Hi']],
            'conversation_history' => [],
            'tokens_used' => 5,
            'version' => 2,
        ]);

        $response = $this->actingAs($user)->getJson('/veda/chat-histories/chat-42');

        $response->assertOk();
        $this->assertSame('Answered chat', $response->json('history.title'));
        $this->assertSame('chat-42', $response->json('history.chatId'));
    }

    public function test_show_returns_404_for_other_users_chat(): void
    {
        $user = $this->createUser();
        $other = $this->createUser(['email' => 'other@example.com']);

        VedaChatHistory::query()->create([
            'user_id' => $other->getAuthIdentifier(),
            'visitor_id' => '',
            'chat_id' => 'chat-secret',
            'title' => 'Private',
            'messages' => [],
            'conversation_history' => [],
            'tokens_used' => 0,
            'version' => 1,
        ]);

        $this->actingAs($user)->getJson('/veda/chat-histories/chat-secret')->assertStatus(404);
    }

    public function test_update_changes_title(): void
    {
        $user = $this->createUser();

        VedaChatHistory::query()->create([
            'user_id' => $user->getAuthIdentifier(),
            'visitor_id' => '',
            'chat_id' => 'chat-7',
            'title' => 'Old title',
            'messages' => [],
            'conversation_history' => [],
            'tokens_used' => 0,
            'version' => 1,
        ]);

        $this->actingAs($user)
            ->patchJson('/veda/chat-histories/chat-7', ['title' => 'New title'])
            ->assertOk()
            ->assertJson(['success' => true]);

        $this->assertSame('New title', VedaChatHistory::query()->where('chat_id', 'chat-7')->value('title'));
    }

    public function test_destroy_deletes_history(): void
    {
        $user = $this->createUser();

        VedaChatHistory::query()->create([
            'user_id' => $user->getAuthIdentifier(),
            'visitor_id' => '',
            'chat_id' => 'chat-9',
            'title' => 'To delete',
            'messages' => [],
            'conversation_history' => [],
            'tokens_used' => 0,
            'version' => 1,
        ]);

        $this->actingAs($user)->deleteJson('/veda/chat-histories/chat-9')->assertOk();

        $this->assertNull(VedaChatHistory::query()->where('chat_id', 'chat-9')->first());
    }
}
