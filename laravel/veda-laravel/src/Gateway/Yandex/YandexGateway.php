<?php

namespace Veda\Laravel\Gateway\Yandex;

use Generator;
use Illuminate\Contracts\Events\Dispatcher;
use Laravel\Ai\Contracts\Files\TranscribableAudio;
use Laravel\Ai\Contracts\Gateway\Gateway;
use Laravel\Ai\Contracts\Providers\AudioProvider;
use Laravel\Ai\Contracts\Providers\EmbeddingProvider;
use Laravel\Ai\Contracts\Providers\ImageProvider;
use Laravel\Ai\Contracts\Providers\TextProvider;
use Laravel\Ai\Contracts\Providers\TranscriptionProvider;
use Laravel\Ai\Gateway\FakeAudioGateway;
use Laravel\Ai\Gateway\FakeImageGateway;
use Laravel\Ai\Gateway\FakeTranscriptionGateway;
use Laravel\Ai\Gateway\StepContext;
use Laravel\Ai\Gateway\StepResponse;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Responses\AudioResponse;
use Laravel\Ai\Responses\EmbeddingsResponse;
use Laravel\Ai\Responses\ImageResponse;
use Laravel\Ai\Responses\TranscriptionResponse;

class YandexGateway implements Gateway
{
    protected YandexTextGateway $textGateway;

    protected YandexEmbeddingGateway $embeddingGateway;

    /**
     * @param  array<string, mixed>  $config
     */
    public function __construct(protected Dispatcher $events, protected array $config)
    {
        $this->textGateway = new YandexTextGateway($events, $config);
        $this->embeddingGateway = new YandexEmbeddingGateway($events, $config);
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
        return $this->textGateway->generateTextStep(
            $provider, $model, $instructions, $messages, $tools, $schema, $options, $timeout, $stepContext
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
        return yield from $this->textGateway->generateStreamStep(
            $invocationId, $provider, $model, $instructions, $messages, $tools, $schema, $options, $timeout, $stepContext
        );
    }

    public function generateEmbeddings(EmbeddingProvider $provider, string $model, array $inputs, int $dimensions, int $timeout = 30, array $providerOptions = []): EmbeddingsResponse
    {
        return $this->embeddingGateway->generateEmbeddings($provider, $model, $inputs, $dimensions, $timeout, $providerOptions);
    }

    public function generateAudio(AudioProvider $provider, string $model, string $text, string $voice, ?string $instructions = null, int $timeout = 30): AudioResponse
    {
        return (new FakeAudioGateway)->generateAudio($provider, $model, $text, $voice, $instructions, $timeout);
    }

    public function generateImage(ImageProvider $provider, string $model, string $prompt, array $attachments = [], ?string $size = null, ?string $quality = null, ?int $timeout = null): ImageResponse
    {
        return (new FakeImageGateway)->generateImage($provider, $model, $prompt, $attachments, $size, $quality, $timeout);
    }

    public function generateTranscription(TranscriptionProvider $provider, string $model, TranscribableAudio $audio, ?string $language = null, bool $diarize = false, int $timeout = 30, array $providerOptions = []): TranscriptionResponse
    {
        return (new FakeTranscriptionGateway)->generateTranscription($provider, $model, $audio, $language, $diarize, $timeout, $providerOptions);
    }
}
