<?php

namespace Veda\Laravel\Gateway\Yandex;

use Generator;
use GuzzleHttp\Client;
use Illuminate\Contracts\Events\Dispatcher;
use Illuminate\JsonSchema\JsonSchemaTypeFactory;
use Illuminate\Support\Str;
use Laravel\Ai\Contracts\Gateway\StepTextGateway;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\Concerns\HandlesFailoverErrors;
use Laravel\Ai\Gateway\Concerns\ParsesServerSentEvents;
use Laravel\Ai\Gateway\StepContext;
use Laravel\Ai\Gateway\StepResponse;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Messages\AssistantMessage;
use Laravel\Ai\Messages\Message;
use Laravel\Ai\Messages\ToolResultMessage;
use Laravel\Ai\ObjectSchema;
use Laravel\Ai\Responses\Data\FinishReason;
use Laravel\Ai\Responses\Data\Meta;
use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\Usage;
use Laravel\Ai\Streaming\Events\ReasoningDelta;
use Laravel\Ai\Streaming\Events\StreamStart;
use Laravel\Ai\Streaming\Events\TextDelta;
use Laravel\Ai\Streaming\Events\TextEnd;
use Laravel\Ai\Streaming\Events\TextStart;
use Laravel\Ai\Tools\ToolNameResolver;
use Veda\Laravel\Gateway\Concerns\AttachesHostScreenshot;
use Veda\Laravel\Gateway\Concerns\InvokesToolsByName;
use Veda\Laravel\Gateway\Concerns\MapsClientTools;
use Veda\Laravel\Gateway\ProviderSession;

class YandexTextGateway implements StepTextGateway
{
    use AttachesHostScreenshot;
    use HandlesFailoverErrors;
    use InvokesToolsByName;
    use MapsClientTools;
    use ParsesServerSentEvents;

    protected YandexAiConfig $yandexConfig;

    /**
     * @param  array<string, mixed>  $config
     */
    public function __construct(protected Dispatcher $events, protected array $config)
    {
        $this->yandexConfig = new YandexAiConfig($config);
        $this->initializeToolCallbacks();
    }

    public function generateTextStep(
        TextProvider $provider,
        string $model,
        ?string $instructions,
        array $messages,
        array $tools,
        ?array $schema,
        ?TextGenerationOptions $options,
        ?int $timeout,
        StepContext $stepContext,
    ): StepResponse {
        $session = $this->createSession($model);

        $payload = [
            'messages' => $this->mapMessages($messages, $instructions, $provider->name(), $model),
            'tools' => $this->mapTools($tools),
        ];

        if ($options?->temperature !== null) {
            $payload['temperature'] = $options->temperature;
        }
        if ($options?->maxTokens !== null) {
            $payload['max_tokens'] = $options->maxTokens;
        }

        $requestPayload = (new YandexResponsesPayloadBuilder($this->yandexConfig))->fromChatPayload(
            $session,
            array_merge($payload, ['stream' => false])
        );

        $resolvedTimeout = $timeout ?? (int) config('veda.stream_timeout', 1800);

        $client = new Client([
            'timeout' => $resolvedTimeout,
            'connect_timeout' => 30,
            'read_timeout' => $resolvedTimeout,
            'headers' => $this->yandexConfig->requestHeaders($session->apiKey, $session->authorizationScheme),
        ]);

        $response = $client->post($this->yandexConfig->chatEndpoint(), [
            'json' => $requestPayload,
        ]);

        $raw = json_decode((string) $response->getBody(), true);
        if (! is_array($raw)) {
            throw new \RuntimeException('Invalid Yandex Responses API response format');
        }

        $accumulator = new YandexResponsesStreamAccumulator;
        $accumulator->applyCompletedResponse($raw);

        $streamResult = $accumulator->toStreamResult();
        $message = $streamResult['message'] ?? [];

        $toolCalls = [];
        foreach ($streamResult['tool_calls'] ?? [] as $toolCallData) {
            $rawArguments = $toolCallData['function']['arguments'] ?? '{}';
            $decodedArguments = is_array($rawArguments)
                ? $rawArguments
                : (json_decode((string) $rawArguments, true) ?? []);

            $toolCalls[] = new ToolCall(
                $toolCallData['id'],
                $toolCallData['function']['name'],
                is_array($decodedArguments) ? $decodedArguments : [],
            );
        }

        return new StepResponse(
            text: (string) ($message['content'] ?? ''),
            toolCalls: $toolCalls,
            finishReason: $toolCalls !== [] ? FinishReason::ToolCalls : FinishReason::Stop,
            usage: new Usage(
                promptTokens: $streamResult['prompt_tokens'] ?? 0,
                completionTokens: $streamResult['completion_tokens'] ?? 0,
            ),
            meta: new Meta($raw['id'] ?? '', $model),
        );
    }

    public function generateStreamStep(
        string $invocationId,
        TextProvider $provider,
        string $model,
        ?string $instructions,
        array $messages,
        array $tools,
        ?array $schema,
        ?TextGenerationOptions $options,
        ?int $timeout,
        StepContext $stepContext,
    ): Generator {
        $session = $this->createSession($model);

        $payload = [
            'messages' => $this->mapMessages($messages, $instructions, $provider->name(), $model),
            'tools' => $this->mapTools($tools),
        ];

        if ($options?->temperature !== null) {
            $payload['temperature'] = $options->temperature;
        }
        if ($options?->maxTokens !== null) {
            $payload['max_tokens'] = $options->maxTokens;
        }

        $resolvedTimeout = $timeout ?? (int) config('veda.stream_timeout', 1800);

        $client = new Client([
            'timeout' => $resolvedTimeout,
            'connect_timeout' => 30,
            'read_timeout' => $resolvedTimeout,
            'headers' => $this->yandexConfig->requestHeaders($session->apiKey, $session->authorizationScheme),
        ]);

        $requestPayload = (new YandexResponsesPayloadBuilder($this->yandexConfig))->fromChatPayload(
            $session,
            array_merge($payload, ['stream' => true])
        );

        $response = $client->post($this->yandexConfig->chatEndpoint(), [
            'json' => $requestPayload,
            'stream' => true,
        ]);

        $streamBody = $response->getBody();
        $messageId = $this->generateEventId();

        yield (new StreamStart(
            $this->generateEventId(),
            $provider->name(),
            $model,
            time(),
        ))->withInvocationId($invocationId);

        $textStartEmitted = false;
        $accumulator = new YandexResponsesStreamAccumulator;
        $previousContentLength = 0;
        $previousReasoningLength = 0;
        $eventType = '';

        while (! $streamBody->eof()) {
            if (connection_aborted()) {
                break;
            }

            $line = trim($this->readLine($streamBody));

            if ($line === '') {
                continue;
            }

            if (str_starts_with($line, 'event:')) {
                $eventType = trim(substr($line, 6));

                continue;
            }

            if (! str_starts_with($line, 'data:')) {
                continue;
            }

            $dataStr = trim(substr($line, 5));
            if ($dataStr === '' || $dataStr === '[DONE]') {
                continue;
            }

            $data = json_decode($dataStr, true);
            if (! is_array($data)) {
                continue;
            }

            $resolvedType = (string) ($data['type'] ?? $eventType);
            $accumulator->applyEvent($resolvedType, $data);

            $reasoning = $accumulator->reasoning();
            if (strlen($reasoning) > $previousReasoningLength) {
                $delta = substr($reasoning, $previousReasoningLength);
                $previousReasoningLength = strlen($reasoning);

                if ($delta !== '') {
                    yield (new ReasoningDelta(
                        $this->generateEventId(),
                        $messageId,
                        $delta,
                        time(),
                    ))->withInvocationId($invocationId);
                }
            }

            $content = $accumulator->content();
            if (strlen($content) > $previousContentLength) {
                if (! $textStartEmitted) {
                    $textStartEmitted = true;
                    yield (new TextStart(
                        $this->generateEventId(),
                        $messageId,
                        time(),
                    ))->withInvocationId($invocationId);
                }

                $delta = substr($content, $previousContentLength);
                $previousContentLength = strlen($content);

                if ($delta !== '') {
                    yield (new TextDelta(
                        $this->generateEventId(),
                        $messageId,
                        $delta,
                        time(),
                    ))->withInvocationId($invocationId);
                }
            }
        }

        if ($textStartEmitted) {
            yield (new TextEnd(
                $this->generateEventId(),
                $messageId,
                time(),
            ))->withInvocationId($invocationId);
        }

        $result = $accumulator->toStreamResult();

        $toolCalls = [];
        foreach ($result['tool_calls'] ?? [] as $toolCallData) {
            $rawArguments = $toolCallData['function']['arguments'] ?? '{}';
            $decodedArguments = is_array($rawArguments)
                ? $rawArguments
                : (json_decode((string) $rawArguments, true) ?? []);

            $toolCall = new ToolCall(
                $toolCallData['id'],
                $toolCallData['function']['name'],
                is_array($decodedArguments) ? $decodedArguments : [],
            );
            $toolCalls[] = $toolCall;

            yield (new \Laravel\Ai\Streaming\Events\ToolCall(
                $this->generateEventId(),
                $toolCall,
                time()
            ))->withInvocationId($invocationId);
        }

        return new StepResponse(
            text: $accumulator->content(),
            toolCalls: $toolCalls,
            finishReason: $toolCalls !== [] ? FinishReason::ToolCalls : FinishReason::Stop,
            usage: new Usage(
                promptTokens: $result['prompt_tokens'] ?? 0,
                completionTokens: $result['completion_tokens'] ?? 0,
            ),
            meta: new Meta('', $model),
        );
    }

    protected function createSession(string $model): ProviderSession
    {
        return new ProviderSession(
            provider: 'yandex',
            model: $model,
            apiKey: $this->yandexConfig->apiKey(),
            apiEndpoint: $this->yandexConfig->chatEndpoint(),
            authorizationScheme: $this->yandexConfig->useIamBearer() ? 'Bearer' : 'Api-Key',
        );
    }

    /**
     * @param  array<int, Message>  $messages
     * @return array<int, array<string, mixed>>
     */
    protected function mapMessages(array $messages, ?string $instructions, ?string $providerName = null, ?string $model = null): array
    {
        $mapped = [];
        if ($instructions) {
            $mapped[] = ['role' => 'system', 'content' => $instructions];
        }

        foreach ($messages as $message) {
            if ($message instanceof ToolResultMessage) {
                foreach ($message->toolResults as $toolResult) {
                    $mapped[] = [
                        'role' => 'tool',
                        'tool_call_id' => $toolResult->resultId ?? $toolResult->id,
                        'content' => is_string($toolResult->result)
                            ? $toolResult->result
                            : json_encode($toolResult->result, JSON_UNESCAPED_UNICODE),
                    ];
                }

                continue;
            }

            if ($message instanceof AssistantMessage) {
                $row = [
                    'role' => 'assistant',
                    'content' => $message->content ?? '',
                ];

                $toolCalls = [];
                foreach ($message->toolCalls ?? [] as $toolCall) {
                    $toolCalls[] = [
                        'id' => $toolCall->id,
                        'type' => 'function',
                        'function' => [
                            'name' => $toolCall->name,
                            'arguments' => json_encode($toolCall->arguments, JSON_UNESCAPED_UNICODE),
                        ],
                    ];
                }

                if ($toolCalls !== []) {
                    $row['tool_calls'] = $toolCalls;
                }

                $mapped[] = $row;

                continue;
            }

            $mapped[] = [
                'role' => $message->role instanceof \BackedEnum ? $message->role->value : (string) $message->role,
                'content' => $message->content ?? '',
            ];
        }

        return $this->attachHostScreenshot(
            $mapped,
            (string) ($providerName ?? 'yandex'),
            (string) ($model ?? ''),
        );
    }

    /**
     * @param  array<int, Tool>  $tools
     * @return array<int, array<string, mixed>>
     */
    protected function mapTools(array $tools): array
    {
        $mapped = [];
        $activeTools = $this->activeToolNames();

        foreach ($tools as $tool) {
            if ($tool instanceof Tool) {
                if (! $this->shouldMapTool($tool, $activeTools)) {
                    continue;
                }

                $schema = $tool->schema(new JsonSchemaTypeFactory);
                $schemaArray = filled($schema)
                    ? (new ObjectSchema($schema))->toSchema()
                    : [];

                $mapped[] = [
                    'type' => 'function',
                    'function' => [
                        'name' => ToolNameResolver::resolve($tool),
                        'description' => (string) $tool->description(),
                        'parameters' => [
                            'type' => 'object',
                            'properties' => $schemaArray['properties'] ?? (object) [],
                            'required' => $schemaArray['required'] ?? [],
                        ],
                    ],
                ];
            }
        }

        return [...$mapped, ...$this->mapClientTools($mapped, 'chat')];
    }

    protected function generateEventId(): string
    {
        return (string) Str::uuid7();
    }
}
