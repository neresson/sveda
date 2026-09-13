<?php

namespace Veda\Laravel\Agent\Middleware;

use Closure;
use Illuminate\Support\Facades\Auth;
use Laravel\Ai\Prompts\AgentPrompt;
use Veda\Laravel\VedaManager;

class AssertTokenQuota
{
    public function handle(AgentPrompt $prompt, Closure $next): mixed
    {
        $user = Auth::guard()->user();

        if ($user) {
            app(VedaManager::class)->getTokenPolicy()->assertRequestAllowed(
                $user,
                (int) config('veda.safety_tokens_per_request', 40000)
            );
        }

        return $next($prompt);
    }
}
