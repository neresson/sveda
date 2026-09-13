<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create(config('veda.tables.chat_histories', 'veda_chat_histories'), function (Blueprint $table) {
            $table->id();
            $table->unsignedBigInteger('user_id')->nullable()->index();
            $table->string('visitor_id', 64)->default('');
            $table->string('chat_id');
            $table->string('title')->nullable();
            $table->json('messages');
            $table->json('conversation_history')->nullable();
            $table->unsignedInteger('tokens_used')->default(0);
            $table->unsignedBigInteger('version')->default(0);
            $table->timestamps();

            $table->unique(['user_id', 'visitor_id', 'chat_id'], 'veda_chat_histories_owner_chat_unique');
            $table->index(['user_id', 'updated_at']);
            $table->index(['visitor_id', 'updated_at'], 'veda_chat_histories_visitor_idx');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists(config('veda.tables.chat_histories', 'veda_chat_histories'));
    }
};
