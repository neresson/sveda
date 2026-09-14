<?php

namespace Veda\Laravel\Agent\Middleware;

use Closure;
use Illuminate\Support\Facades\Request;
use Illuminate\Support\Str;
use Laravel\Ai\Prompts\AgentPrompt;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Http\Middleware\VedaEmbedAuth;
use Veda\Laravel\Services\PageContextSanitizer;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Support\FrontendToolContinuation;

class BindRequestContext
{
    public function handle(AgentPrompt $prompt, Closure $next): mixed
    {
        $pageContext = (new PageContextSanitizer)->sanitize(Request::input('context', []));

        $chatId = Request::input('chatId');
        $chatId = is_string($chatId) && trim($chatId) !== '' ? trim($chatId) : null;

        $isEmbedGuest = request()->attributes->get(VedaEmbedAuth::ATTRIBUTE_EMBED_GUEST) === true;
        $isEmbedMode = $isEmbedGuest || request()->attributes->get(VedaEmbedAuth::ATTRIBUTE_EMBED_MODE) === true;

        $userId = null;
        if ($prompt->agent instanceof VedaAgent && $prompt->agent->user) {
            $userId = $prompt->agent->user->getAuthIdentifier();
        }

        $continuation = FrontendToolContinuation::fromMessages(
            is_array(Request::input('messages')) ? Request::input('messages') : []
        );

        RequestContext::bind(
            $pageContext,
            $isEmbedMode,
            $userId,
            $chatId,
            (string) Str::uuid7(),
            $isEmbedGuest,
            $this->clientTools(),
            $continuation['toolCalls'] ?? null,
            $continuation['toolResults'] ?? null,
            Request::has('options.thinking') ? Request::boolean('options.thinking') : null,
        );

        app()->terminating(function () {
            RequestContext::forget();
        });

        return $next($prompt);
    }

    /**
     * @return array<int, array{name: string, description: string, parameters: array<string, mixed>}>
     */
    protected function clientTools(): array
    {
        $tools = Request::input('clientTools', []);
        if (! is_array($tools)) {
            return [];
        }

        $mapped = [];
        foreach ($tools as $tool) {
            if (! is_array($tool)) {
                continue;
            }

            $name = $tool['name'] ?? null;
            if (! is_string($name) || trim($name) === '') {
                continue;
            }

            $parameters = $tool['parameters'] ?? null;

            $mapped[] = [
                'name' => trim($name),
                'description' => is_string($tool['description'] ?? null) ? $tool['description'] : '',
                'parameters' => is_array($parameters) ? $parameters : ['type' => 'object', 'properties' => new \stdClass],
            ];
        }

        return $mapped;
    }
}
