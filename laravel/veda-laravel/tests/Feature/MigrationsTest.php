<?php

namespace Veda\Laravel\Tests\Feature;

use Illuminate\Support\Facades\Schema;
use Veda\Laravel\Models\VedaChatCompaction;
use Veda\Laravel\Models\VedaChatHistory;
use Veda\Laravel\Models\VedaChatTurn;
use Veda\Laravel\Models\VedaGeneration;
use Veda\Laravel\Models\VedaSetting;
use Veda\Laravel\Tests\TestCase;

class MigrationsTest extends TestCase
{
    public function test_veda_tables_are_created(): void
    {
        $this->assertTrue(Schema::hasTable(config('veda.tables.chat_histories')));
        $this->assertTrue(Schema::hasTable(config('veda.tables.chat_turns')));
        $this->assertTrue(Schema::hasTable(config('veda.tables.chat_compactions')));
        $this->assertTrue(Schema::hasTable(config('veda.tables.generations')));
        $this->assertTrue(Schema::hasTable(config('veda.tables.settings')));
        $this->assertTrue(Schema::hasTable(config('veda.tables.code_sources')));
        $this->assertTrue(Schema::hasTable(config('veda.tables.code_index_chunks')));
        $this->assertTrue(Schema::hasColumn((new VedaGeneration)->getTable(), 'model'));
    }

    public function test_table_names_come_from_config(): void
    {
        $this->assertSame('veda_chat_histories', config('veda.tables.chat_histories'));
        $this->assertSame('veda_chat_turns', config('veda.tables.chat_turns'));
        $this->assertSame('veda_chat_compactions', config('veda.tables.chat_compactions'));
        $this->assertSame('veda_generations', config('veda.tables.generations'));
        $this->assertSame('veda_settings', config('veda.tables.settings'));
        $this->assertSame('veda_code_sources', config('veda.tables.code_sources'));
        $this->assertSame('veda_code_index_chunks', config('veda.tables.code_index_chunks'));
    }

    public function test_models_use_configured_table_names(): void
    {
        $this->assertSame('veda_chat_histories', (new VedaChatHistory)->getTable());
        $this->assertSame('veda_chat_turns', (new VedaChatTurn)->getTable());
        $this->assertSame('veda_chat_compactions', (new VedaChatCompaction)->getTable());
        $this->assertSame('veda_generations', (new VedaGeneration)->getTable());
        $this->assertSame('veda_settings', (new VedaSetting)->getTable());
    }
}
