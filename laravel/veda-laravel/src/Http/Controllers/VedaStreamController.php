<?php

namespace Veda\Laravel\Http\Controllers;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Auth;
use Laravel\Ai\Responses\StreamableAgentResponse;
use Laravel\Ai\Responses\StreamedAgentResponse;
use Symfony\Component\HttpFoundation\Response;
use Veda\Laravel\Agent\VedaAgent;
use Veda\Laravel\Exceptions\VedaTokenLimitExceededException;
use Veda\Laravel\Jobs\CompactConversationJob;
use Veda\Laravel\Jobs\GenerateChatTitleJob;
use Veda\Laravel\Models\VedaGeneration;
use Veda\Laravel\Services\ChatHistoryCheckpointService;
use Veda\Laravel\Services\EmbedChatScope;
use Veda\Laravel\Services\MessageContent;
use Veda\Laravel\Streaming\VedaWireProtocolMapper;
use Veda\Laravel\Streaming\VercelDataProtocolMapper;
use Veda\Laravel\Support\FrontendToolContinuation;
use Veda\Laravel\VedaManager;

class VedaStreamController
{
    public function stream(Request $request): Response
    {
        $validated = $this->validatePayload($request);
        $user = $request->user() ?? Auth::guard()->user();

        $limitResponse = $this->assertQuota($user);
        if ($limitResponse !== null) {
            return $limitResponse;
        }

        $generation = VedaGeneration::create([
            'user_id' => $user?->getAuthIdentifier(),
            'generation_type' => 'chat',
            'prompt' => $validated['prompt'] ?? 'Continue conversation',
            'status' => VedaGeneration::STATUS_PENDING,
        ]);

        $agent = $this->makeAgent($user, $validated['chatId'] ?? null);

        $streamTimeout = (int) config('veda.stream_timeout', 1800);
        set_time_limit($streamTimeout);
        ignore_user_abort(true);

        $providerToUse = $validated['provider'] ?? config('veda.provider');
        $modelToUse = $validated['model'] ?? config('veda.model');

        $stream = $agent->stream(
            $validated['prompt'] ?? 'Continue conversation',
            provider: $providerToUse,
            model: $modelToUse,
            timeout: $streamTimeout,
        );

        $historyScope = $this->historyScope($request);

        $stream->then(function (StreamedAgentResponse $response) use ($generation, $validated, $agent, $user, $historyScope, $providerToUse, $modelToUse) {
            $tokensUsed = $response->usage->promptTokens + $response->usage->completionTokens;

            $result = [
                'explanation' => $this->applyResponseGuard(
                    (string) ($response->text ?? ''),
                    $validated,
                    $providerToUse,
                    $modelToUse,
                ),
                'tokens_used' => $tokensUsed,
                'chat_id' => $agent->currentConversation(),
            ];

            $generation->markAsCompleted($result, $tokensUsed);

            if ($user) {
                app(VedaManager::class)->getTokenPolicy()->recordUsage(
                    $user,
                    $this->resolvePrimaryProviderName($providerToUse) ?? '',
                    (string) ($modelToUse ?? ''),
                    $response->usage->promptTokens,
                    $response->usage->completionTokens,
                );
            }

            $checkpointChatId = $validated['chatId'] ?? $agent->currentConversation();
            if ($user && is_string($checkpointChatId) && $checkpointChatId !== '') {
                app(ChatHistoryCheckpointService::class)->checkpointAfterAgentStream(
                    $user,
                    $checkpointChatId,
                    ['tokensUsed' => $tokensUsed],
                    $historyScope,
                );
            }

            if ($user && ! $historyScope->embedVisitorMode) {
                $this->dispatchChatTitleGeneration($user, $validated, $checkpointChatId);
            }

            if ($user && $agent->currentConversation() && ! $historyScope->embedVisitorMode) {
                CompactConversationJob::dispatch(
                    $user->getAuthIdentifier(),
                    $agent->currentConversation(),
                )->afterResponse();
            }
        });

        return $this->toStreamResponse($stream, $validated['chatId'] ?? null, $this->wantsVedaProtocol($request));
    }

    public function message(Request $request): JsonResponse
    {
        $validated = $this->validatePayload($request);
        $user = $request->user() ?? Auth::guard()->user();

        $limitResponse = $this->assertQuota($user);
        if ($limitResponse !== null) {
            return $limitResponse;
        }

        $generation = VedaGeneration::create([
            'user_id' => $user?->getAuthIdentifier(),
            'generation_type' => 'chat',
            'prompt' => $validated['prompt'] ?? 'Continue conversation',
            'status' => VedaGeneration::STATUS_PENDING,
        ]);

        $historyScope = $this->historyScope($request);

        try {
            $agent = $this->makeAgent($user, $validated['chatId'] ?? null);

            $providerToUse = $validated['provider'] ?? config('veda.provider');
            $modelToUse = $validated['model'] ?? config('veda.model');

            $response = $agent->prompt(
                $validated['prompt'] ?? 'Continue conversation',
                provider: $providerToUse,
                model: $modelToUse,
            );

            $tokensUsed = $response->usage->promptTokens + $response->usage->completionTokens;

            $result = [
                'explanation' => $this->applyResponseGuard(
                    (string) ($response->text ?? ''),
                    $validated,
                    $providerToUse,
                    $modelToUse,
                ),
                'tokens_used' => $tokensUsed,
                'chat_id' => $agent->currentConversation(),
            ];

            $generation->markAsCompleted($result, $tokensUsed);

            if ($user) {
                app(VedaManager::class)->getTokenPolicy()->recordUsage(
                    $user,
                    $this->resolvePrimaryProviderName($providerToUse) ?? '',
                    (string) ($modelToUse ?? ''),
                    $response->usage->promptTokens,
                    $response->usage->completionTokens,
                );
            }

            $checkpointChatId = $validated['chatId'] ?? $agent->currentConversation();
            if ($user && is_string($checkpointChatId) && $checkpointChatId !== '') {
                app(ChatHistoryCheckpointService::class)->checkpointAfterAgentStream(
                    $user,
                    $checkpointChatId,
                    ['tokensUsed' => $tokensUsed],
                    $historyScope,
                );
            }

            if ($user && ! $historyScope->embedVisitorMode) {
                $this->dispatchChatTitleGeneration($user, $validated, $checkpointChatId);
            }

            if ($user && $agent->currentConversation() && ! $historyScope->embedVisitorMode) {
                CompactConversationJob::dispatch(
                    $user->getAuthIdentifier(),
                    $agent->currentConversation(),
                )->afterResponse();
            }

            return response()->json($result);
        } catch (\Throwable $e) {
            $generation->markAsFailed($e->getMessage());

            return response()->json([
                'message' => $e->getMessage(),
            ], 500);
        }
    }

    protected function makeAgent(?Authenticatable $user, ?string $chatId): VedaAgent
    {
        return new VedaAgent($user, $chatId);
    }

    protected function historyScope(Request $request): EmbedChatScope
    {
        return EmbedChatScope::fromRequest($request);
    }

    /**
     * @param  array<string, mixed>  $validated
     */
    protected function applyResponseGuard(string $text, array $validated, array|string|null $provider, ?string $model): string
    {
        return app(VedaManager::class)->applyResponseGuard($text, [
            'prompt' => (string) ($validated['prompt'] ?? ''),
            'provider' => $this->resolvePrimaryProviderName($provider),
            'model' => $model,
        ]);
    }

    protected function assertQuota(?Authenticatable $user): ?JsonResponse
    {
        if (! $user) {
            return null;
        }

        try {
            app(VedaManager::class)->getTokenPolicy()->assertRequestAllowed(
                $user,
                (int) config('veda.safety_tokens_per_request', 40000)
            );

            return null;
        } catch (VedaTokenLimitExceededException $e) {
            return response()->json([
                'error' => 'token_limit_exceeded',
                'message' => $e->getMessage(),
                'period' => $e->period,
                'mode' => $e->mode,
            ], 403);
        }
    }

    /**
     * @return array<string, mixed>
     */
    protected function validatePayload(Request $request): array
    {
        $validated = $request->validate([
            'prompt' => 'nullable|string|max:200000',
            'messages' => 'nullable|array',
            'chatId' => 'nullable|string|max:191',
            'context' => 'nullable|array',
            'clientTools' => 'nullable|array|max:50',
            'clientTools.*.name' => 'required_with:clientTools|string|max:100',
            'clientTools.*.description' => 'nullable|string|max:4000',
            'clientTools.*.parameters' => 'nullable|array',
            'provider' => 'nullable|string|max:50',
            'model' => 'nullable|string|max:100',
            'options' => 'nullable|array',
            'options.thinking' => 'nullable|boolean',
        ]);

        $messages = $validated['messages'] ?? null;
        if (is_array($messages) && $messages !== []) {
            $messages = $this->withoutTrailingEmptyAssistant(array_values($messages));
            $lastMessage = $messages !== [] ? $messages[array_key_last($messages)] : null;
            $lastRole = is_array($lastMessage) ? ($lastMessage['role'] ?? '') : '';

            if ($lastRole === 'user' && is_array($lastMessage)) {
                $validated['prompt'] = $this->extractUserPrompt($lastMessage);
                $validated['conversation_history'] = array_slice($messages, 0, -1);
            } elseif ($lastRole === 'tool'
                && FrontendToolContinuation::fromMessages($messages) !== null) {
                $validated['prompt'] = '';
            }
        }

        return $validated;
    }

    /**
     * @param  array<int, mixed>  $messages
     * @return array<int, mixed>
     */
    protected function withoutTrailingEmptyAssistant(array $messages): array
    {
        while ($messages !== []) {
            $last = $messages[array_key_last($messages)];
            if (! is_array($last) || ($last['role'] ?? '') !== 'assistant') {
                break;
            }

            if ($this->extractUserPrompt($last) !== '') {
                break;
            }

            array_pop($messages);
        }

        return array_values($messages);
    }

    /**
     * @param  array<string, mixed>  $message
     */
    protected function extractUserPrompt(array $message): string
    {
        $text = trim(MessageContent::toText($message['content'] ?? ''));
        if ($text !== '') {
            return $text;
        }

        $parts = $message['parts'] ?? null;
        if (! is_array($parts)) {
            return '';
        }

        $texts = [];
        foreach ($parts as $part) {
            if (is_array($part) && ($part['type'] ?? '') === 'text') {
                $partText = trim((string) ($part['text'] ?? ''));
                if ($partText !== '') {
                    $texts[] = $partText;
                }
            }
        }

        return implode("\n", $texts);
    }

    protected function wantsVedaProtocol(Request $request): bool
    {
        $accept = (string) $request->header('Accept', '');
        if (str_contains($accept, 'application/vnd.veda.stream+json')) {
            return true;
        }
        if (str_contains($accept, 'text/event-stream') && $request->header('X-Veda-Protocol') === 'vercel') {
            return false;
        }

        $configured = (string) config('veda.protocol', 'veda');

        if ($request->header('X-Veda-Protocol') === 'veda') {
            return true;
        }

        return $configured === 'veda';
    }

    protected function toStreamResponse(StreamableAgentResponse $stream, ?string $chatId, bool $vedaProtocol): Response
    {
        $vedaMapper = new VedaWireProtocolMapper($chatId);
        $vercelMapper = new VercelDataProtocolMapper;

        return response()->stream(function () use ($stream, $vedaMapper, $vercelMapper, $vedaProtocol) {
            foreach ($stream as $event) {
                $vedaEvent = $vedaMapper->map($event);
                if ($vedaEvent === null) {
                    continue;
                }

                $payload = $vedaProtocol ? $vedaEvent : $vercelMapper->map($vedaEvent);
                if ($payload === null) {
                    continue;
                }

                echo 'data: '.json_encode($payload, JSON_UNESCAPED_UNICODE)."\n\n";
                if (ob_get_level() > 0) {
                    ob_flush();
                }
                flush();
            }

            echo "data: [DONE]\n\n";
            if (ob_get_level() > 0) {
                ob_flush();
            }
            flush();
        }, 200, [
            'Content-Type' => 'text/event-stream',
            'Cache-Control' => 'no-cache',
            'X-Accel-Buffering' => 'no',
            'X-Veda-Protocol-Version' => VedaWireProtocolMapper::VERSION,
        ]);
    }

    protected function resolvePrimaryProviderName(array|string|null $provider): ?string
    {
        if (is_array($provider)) {
            $first = reset($provider);

            return is_string($first) ? $first : null;
        }

        return $provider;
    }

    protected function dispatchChatTitleGeneration(
        Authenticatable $user,
        array $validated,
        mixed $checkpointChatId,
    ): void {
        if (! (bool) config('veda.title_generation.enabled', true)) {
            return;
        }

        $chatId = is_string($checkpointChatId) && $checkpointChatId !== ''
            ? $checkpointChatId
            : (is_string($validated['chatId'] ?? null) ? $validated['chatId'] : '');

        $userRequest = trim((string) ($validated['prompt'] ?? ''));
        if ($chatId === '' || $userRequest === '') {
            return;
        }

        GenerateChatTitleJob::dispatch(
            $user->getAuthIdentifier(),
            $chatId,
            $userRequest,
        );
    }
}
