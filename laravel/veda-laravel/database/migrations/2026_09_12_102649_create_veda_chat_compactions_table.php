<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create(config('veda.tables.chat_compactions', 'veda_chat_compactions'), function (Blueprint $table) {
            $table->id();
            $table->unsignedBigInteger('user_id')->nullable()->index();
            $table->string('chat_id');
            $table->text('summary_text');
            $table->string('transcript_path');
            $table->string('source_fingerprint', 64);
            $table->timestamp('summarized_at');
            $table->timestamps();

            $table->unique(['user_id', 'chat_id']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists(config('veda.tables.chat_compactions', 'veda_chat_compactions'));
    }
};
