<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        $sources = config('veda.tables.code_sources', 'veda_code_sources');
        $chunks = config('veda.tables.code_index_chunks', 'veda_code_index_chunks');

        Schema::create($chunks, function (Blueprint $table) use ($sources) {
            $table->id();
            $table->foreignId('code_source_id')->constrained($sources)->cascadeOnDelete();
            $table->string('path', 2048);
            $table->string('chunk_kind', 16)->default('lines');
            $table->string('symbol_name')->nullable();
            $table->string('qualified_name')->nullable();
            $table->string('language', 32)->nullable();
            $table->unsignedInteger('chunk_index')->default(0);
            $table->unsignedInteger('start_line')->default(1);
            $table->unsignedInteger('end_line')->default(1);
            $table->longText('content');
            $table->string('content_hash', 64)->nullable();
            $table->longText('embedding')->nullable();
            $table->timestamps();

            $table->index('code_source_id');
            $table->index(['code_source_id', 'chunk_kind']);
            $table->index('content_hash');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists(config('veda.tables.code_index_chunks', 'veda_code_index_chunks'));
    }
};
