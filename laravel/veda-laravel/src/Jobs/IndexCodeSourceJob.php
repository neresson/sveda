<?php

namespace Veda\Laravel\Jobs;

use Illuminate\Bus\Queueable;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Bus\Dispatchable;
use Illuminate\Queue\InteractsWithQueue;
use Illuminate\Queue\MaxAttemptsExceededException;
use Illuminate\Queue\Middleware\WithoutOverlapping;
use Illuminate\Queue\SerializesModels;
use Illuminate\Support\Facades\Cache;
use Throwable;
use Veda\Laravel\CodeIndex\CodeSourceIndexer;
use Veda\Laravel\Models\VedaCodeSource;
use Veda\Laravel\Support\Utf8Text;

class IndexCodeSourceJob implements ShouldQueue
{
    use Dispatchable, InteractsWithQueue, Queueable, SerializesModels;

    public int $timeout = 3600;

    public int $tries = 250;

    public function __construct(public int $codeSourceId)
    {
        $this->onQueue((string) config('veda.code_index.queue', 'default'));
    }

    public static function releaseOverlapLock(int $codeSourceId): void
    {
        $job = new self($codeSourceId);
        foreach ($job->middleware() as $middleware) {
            if ($middleware instanceof WithoutOverlapping) {
                Cache::lock($middleware->getLockKey($job))->forceRelease();

                return;
            }
        }
    }

    public function middleware(): array
    {
        return [
            (new WithoutOverlapping((string) $this->codeSourceId))
                ->releaseAfter(20)
                ->expireAfter(3700),
        ];
    }

    public function handle(CodeSourceIndexer $indexer): void
    {
        $source = VedaCodeSource::query()->find($this->codeSourceId);
        if (! $source) {
            return;
        }
        if ($source->status === 'cancelled' || $source->index_cancel_requested) {
            return;
        }

        $indexer->index($source);
    }

    public function failed(?Throwable $exception): void
    {
        $source = VedaCodeSource::query()->find($this->codeSourceId);
        if (! $source) {
            return;
        }
        if (! in_array($source->status, ['pending', 'indexing'], true)) {
            return;
        }
        $msg = 'queue_job_failed';
        if ($exception instanceof MaxAttemptsExceededException) {
            $msg = 'code_index_max_queue_attempts';
        } elseif ($exception) {
            $msg = Utf8Text::forErrorColumn($exception);
        }
        $source->update([
            'status' => 'failed',
            'error_message' => Utf8Text::sanitize($msg, 500),
            'indexing_progress' => 0,
            'indexing_phase' => null,
            'index_cancel_requested' => false,
        ]);
    }
}
