<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table(config('veda.tables.generations', 'veda_generations'), function (Blueprint $table) {
            $table->string('model')->nullable()->index()->after('generation_type');
        });
    }

    public function down(): void
    {
        Schema::table(config('veda.tables.generations', 'veda_generations'), function (Blueprint $table) {
            $table->dropIndex(['model']);
            $table->dropColumn('model');
        });
    }
};
