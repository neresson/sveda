<?php

namespace Veda\Laravel\Agent;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Support\Facades\Auth;
use Laravel\Ai\Concerns\RemembersConversations;
use Laravel\Ai\Contracts\Agent;
use Laravel\Ai\Contracts\Conversational;
use Laravel\Ai\Contracts\HasMiddleware;
use Laravel\Ai\Contracts\HasProviderOptions;
use Laravel\Ai\Contracts\HasTools;
use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Enums\Lab;
use Laravel\Ai\Messages\AssistantMessage;
use Laravel\Ai\Messages\ToolResultMessage;
use Laravel\Ai\Promptable;
use Stringable;
use Veda\Laravel\Agent\Middleware\AssertTokenQuota;
use Veda\Laravel\Agent\Middleware\BindRequestContext;
use Veda\Laravel\Agent\Middleware\RunSearchPreflight;
use Veda\Laravel\Prompts\PromptBuilder;
use Veda\Laravel\Services\ConversationCompactionStore;
use Veda\Laravel\Services\HostMcpToolGateway;
use Veda\Laravel\Services\RequestContext;
use Veda\Laravel\Tools\SpawnParallelTasksTool;
use Veda\Laravel\Tools\ToolResolver;
use Veda\Laravel\VedaManager;

class VedaAgent implements Agent, Conversational, HasMiddleware, HasProviderOptions, HasTools
{
    use Promptable;
    use RemembersConversations {
        messages as protected conversationMessages;
    }

    public ?Authenticatable $user = null;

    public function __construct(
        ?Authenticatable $user = null,
        ?string $chatId = null,
    ) {
        $this->user = $user ?? Auth::guard()->user();

        if ($this->user && $chatId) {
            $this->continue($chatId, as: $this->user);
        } elseif ($this->user) {
            $this->forUser($this->user);
        }
    }

    public function instructions(): Stringable|string
    {
        $manager = app(VedaManager::class);
        $requestContext = RequestContext::current();
        $pageContext = $requestContext?->promptPageContext() ?? [];

        $resolvedTools = $this->resolvedSessionTools($requestContext);

        $contextSections = [];
        foreach ($manager->contextProviders() as $provider) {
            $section = $provider($this->user, $pageContext, $requestContext);
            if (is_string($section) && trim($section) !== '') {
                $contextSections[] = trim($section);
            }
        }

        $summary = null;
        if ($this->user && $this->currentConversation()) {
            $frozen = app(ConversationCompactionStore::class)
                ->findLatestSummary($this->user, $this->currentConversation());
            if ($frozen && $frozen->summary_text) {
                $summary = $frozen->summary_text;
            }
        }

        return app(PromptBuilder::class)->buildSystemPrompt(
            $pageContext,
            $resolvedTools,
            [],
            $manager->resolveGlobalContextBlock($this->user, $pageContext, $requestContext),
            $contextSections,
            $summary,
        );
    }

    /**
     * @return array<int, Tool>
     */
    public function tools(): iterable
    {
        $manager = app(VedaManager::class);
        $requestContext = RequestContext::current();

        $tools = $this->resolvedSessionTools($requestContext);

        if ($manager->getSubagentResolver() !== null) {
            $tools[] = new SpawnParallelTasksTool($this->user);
        }

        return $tools;
    }

    /**
     * @return array<int, Tool>
     */
    protected function resolvedSessionTools(?RequestContext $requestContext): array
    {
        $resolved = app(ToolResolver::class)->resolve($this->user, $requestContext);

        return [...$resolved['tools'], ...app(HostMcpToolGateway::class)->tools($requestContext)];
    }

    /**
     * @return iterable<int, object>
     */
    public function messages(): iterable
    {
        $messages = collect($this->conversationMessages());

        $context = RequestContext::current();
        if ($context !== null && $context->hasPendingToolContinuation()) {
            $messages->push(new AssistantMessage('', collect($context->pendingToolCalls ?? [])));
            $messages->push(new ToolResultMessage(collect($context->pendingToolResults ?? [])));
        }

        return $messages->all();
    }

    public function maxSteps(): int
    {
        return max(1, (int) config('veda.max_steps', 30));
    }

    /**
     * @return array<int, object>
     */
    public function middleware(): array
    {
        return [
            new AssertTokenQuota,
            new BindRequestContext,
            new RunSearchPreflight,
        ];
    }

    public function timeout(): int
    {
        return (int) config('veda.stream_timeout', 1800);
    }

    public function providerOptions(Lab|string $provider): array
    {
        $driver = $provider instanceof Lab ? $provider->value : (string) $provider;
        $thinkingEnabled = RequestContext::current()?->thinkingEnabled === true;

        return match ($driver) {
            'veda-responses' => [
                'reasoning' => [
                    'effort' => $thinkingEnabled ? 'high' : 'none',
                ],
            ],
            'veda-anthropic' => $thinkingEnabled
                ? [
                    'thinking' => [
                        'type' => 'enabled',
                        'budget_tokens' => 8192,
                    ],
                ]
                : [
                    'thinking' => [
                        'type' => 'disabled',
                    ],
                ],
            default => [],
        };
    }
}
