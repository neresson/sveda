<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create(config('veda.tables.chat_turns', 'veda_chat_turns'), function (Blueprint $table) {
            $table->id();
            $table->unsignedBigInteger('user_id')->nullable()->index();
            $table->string('chat_id');
            $table->unsignedInteger('turn_index');
            $table->text('frozen_user_content');
            $table->text('global_context_rendered')->nullable();
            $table->string('global_context_cache_key', 64)->nullable();
            $table->timestamps();

            $table->unique(['user_id', 'chat_id', 'turn_index']);
            $table->index(['user_id', 'chat_id']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists(config('veda.tables.chat_turns', 'veda_chat_turns'));
    }
};
