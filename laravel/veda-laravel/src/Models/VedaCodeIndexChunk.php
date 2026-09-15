<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

class VedaCodeIndexChunk extends Model
{
    protected $fillable = [
        'code_source_id',
        'path',
        'chunk_kind',
        'symbol_name',
        'qualified_name',
        'language',
        'chunk_index',
        'start_line',
        'end_line',
        'content',
        'content_hash',
        'embedding',
    ];

    protected $hidden = [
        'embedding',
    ];

    public function getTable(): string
    {
        return config('veda.tables.code_index_chunks', 'veda_code_index_chunks');
    }

    /**
     * @return BelongsTo<VedaCodeSource, $this>
     */
    public function codeSource(): BelongsTo
    {
        return $this->belongsTo(VedaCodeSource::class, 'code_source_id');
    }
}
