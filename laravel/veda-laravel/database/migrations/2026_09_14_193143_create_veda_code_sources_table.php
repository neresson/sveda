<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create(config('veda.tables.code_sources', 'veda_code_sources'), function (Blueprint $table) {
            $table->id();
            $table->string('name');
            $table->text('description')->nullable();
            $table->string('provider', 32);
            $table->text('local_absolute_path')->nullable();
            $table->text('git_remote_url')->nullable();
            $table->string('git_branch', 190)->default('main');
            $table->text('git_clone_token')->nullable();
            $table->string('clone_relative_path', 512)->nullable();
            $table->string('status', 32)->default('configuring');
            $table->unsignedTinyInteger('indexing_progress')->default(0);
            $table->string('indexing_phase', 32)->nullable();
            $table->boolean('index_cancel_requested')->default(false);
            $table->text('error_message')->nullable();
            $table->longText('structure_summary')->nullable();
            $table->json('metadata')->nullable();
            $table->timestamp('last_indexed_at')->nullable();
            $table->timestamps();

            $table->index('status');
            $table->index('provider');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists(config('veda.tables.code_sources', 'veda_code_sources'));
    }
};
