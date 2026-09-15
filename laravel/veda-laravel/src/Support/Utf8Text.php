<?php

namespace Veda\Laravel\Support;

use Illuminate\Database\QueryException;
use Throwable;

final class Utf8Text
{
    public static function sanitize(?string $text, int $maxBytes = 1_048_576): string
    {
        if ($text === null || $text === '') {
            return '';
        }
        $converted = @iconv('UTF-8', 'UTF-8//IGNORE', $text);
        if ($converted === false) {
            return '';
        }
        $converted = str_replace("\0", '', $converted);
        if (function_exists('mb_scrub')) {
            $converted = mb_scrub($converted, 'UTF-8');
        }
        if ($maxBytes > 0 && strlen($converted) > $maxBytes) {
            $converted = substr($converted, 0, $maxBytes);
        }

        return $converted;
    }

    public static function forErrorColumn(Throwable $e): string
    {
        if ($e instanceof QueryException) {
            $sqlState = (string) ($e->errorInfo[0] ?? '');
            $msg = $e->getMessage();
            if ($sqlState === '22021' || str_contains($msg, 'invalid byte sequence for encoding')) {
                return 'code_index_invalid_utf8';
            }
            if ($sqlState !== '') {
                return 'code_index_database_error:'.$sqlState;
            }

            return 'code_index_database_error';
        }

        $msg = self::sanitize($e->getMessage(), 500);

        return $msg !== '' ? $msg : 'queue_job_failed';
    }

    public static function forLog(Throwable $e, int $maxBytes = 4000): string
    {
        if ($e instanceof QueryException) {
            $prev = $e->getPrevious();
            if ($prev instanceof \PDOException) {
                return self::sanitize($prev->getMessage(), $maxBytes);
            }
            $state = (string) ($e->errorInfo[0] ?? '');

            return self::sanitize($state !== '' ? $state : 'query_exception', $maxBytes);
        }

        return self::sanitize($e->getMessage(), $maxBytes);
    }
}
