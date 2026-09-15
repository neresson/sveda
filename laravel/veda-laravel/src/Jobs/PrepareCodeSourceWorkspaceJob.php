<?php

namespace Veda\Laravel\Jobs;

use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\Middleware\WithoutOverlapping;
use Illuminate\Queue\SerializesModels;
use Veda\Laravel\CodeIndex\CodeSourceIndexer;
use Veda\Laravel\Models\VedaCodeSource;

class PrepareCodeSourceWorkspaceJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $timeout = 1200;

    public function __construct(public int $codeSourceId)
    {
        $this->onQueue((string) config('veda.code_index.queue', 'default'));
    }

    public function middleware(): array
    {
        return [
            (new WithoutOverlapping('prepare-'.$this->codeSourceId))
                ->releaseAfter(20)
                ->expireAfter(1300),
        ];
    }

    public function handle(CodeSourceIndexer $indexer): void
    {
        $source = VedaCodeSource::query()->find($this->codeSourceId);
        if (! $source || $source->status !== 'configuring') {
            return;
        }

        $indexer->prepareWorkspace($source);
    }
}
