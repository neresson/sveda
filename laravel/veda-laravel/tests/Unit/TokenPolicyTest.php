<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Exceptions\VedaTokenLimitExceededException;
use Veda\Laravel\Support\NullTokenPolicy;
use Veda\Laravel\Tests\TestCase;

class TokenPolicyTest extends TestCase
{
    public function test_null_policy_allows_any_request(): void
    {
        $policy = new NullTokenPolicy;

        $policy->assertRequestAllowed(null, PHP_INT_MAX);

        $this->assertTrue(true);
    }

    public function test_null_policy_record_usage_is_noop(): void
    {
        $policy = new NullTokenPolicy;

        $policy->recordUsage(null, 'openai', 'gpt-5', 100, 200);

        $this->assertTrue(true);
    }

    public function test_limit_exception_carries_period_and_mode(): void
    {
        $exception = new VedaTokenLimitExceededException('Limit reached', 'day', 'hard');

        $this->assertSame('day', $exception->period);
        $this->assertSame('hard', $exception->mode);
        $this->assertSame('Limit reached', $exception->getMessage());
    }
}
