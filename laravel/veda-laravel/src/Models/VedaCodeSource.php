<?php

namespace Veda\Laravel\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;

class VedaCodeSource extends Model
{
    protected $fillable = [
        'name',
        'description',
        'provider',
        'local_absolute_path',
        'git_remote_url',
        'git_branch',
        'git_clone_token',
        'clone_relative_path',
        'status',
        'indexing_progress',
        'indexing_phase',
        'index_cancel_requested',
        'error_message',
        'structure_summary',
        'metadata',
        'last_indexed_at',
    ];

    protected $hidden = [
        'git_clone_token',
    ];

    protected function casts(): array
    {
        return [
            'metadata' => 'array',
            'last_indexed_at' => 'datetime',
            'git_clone_token' => 'encrypted',
            'index_cancel_requested' => 'boolean',
        ];
    }

    public function getTable(): string
    {
        return config('veda.tables.code_sources', 'veda_code_sources');
    }

    /**
     * @return HasMany<VedaCodeIndexChunk, $this>
     */
    public function chunks(): HasMany
    {
        return $this->hasMany(VedaCodeIndexChunk::class, 'code_source_id');
    }
}
