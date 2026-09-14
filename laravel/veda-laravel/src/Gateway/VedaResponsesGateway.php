<?php

namespace Veda\Laravel\Gateway;

use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\OpenAi\OpenAiGateway;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Messages\Message;
use Laravel\Ai\Messages\UserMessage;
use Laravel\Ai\Providers\Provider;
use Veda\Laravel\Gateway\Concerns\AttachesHostScreenshot;
use Veda\Laravel\Gateway\Concerns\InvokesToolsByName;
use Veda\Laravel\Gateway\Concerns\MapsActiveTools;
use Veda\Laravel\Gateway\Concerns\MapsClientTools;

class VedaResponsesGateway extends OpenAiGateway
{
    use AttachesHostScreenshot;
    use InvokesToolsByName;
    use MapsActiveTools;
    use MapsClientTools;

    protected ?Provider $activeProvider = null;

    protected function isStateless(Provider $provider): bool
    {
        return true;
    }

    protected function isReasoningModel(string $model): bool
    {
        return false;
    }

    protected function mapTools(array $tools, Provider $provider): array
    {
        $mapped = [];
        $activeTools = $this->activeToolNames();

        foreach ($tools as $tool) {
            if ($tool instanceof Tool && $this->shouldMapTool($tool, $activeTools)) {
                $mapped[] = $this->mapTool($tool);
            }
        }

        return [...$mapped, ...$this->mapClientTools($mapped, 'responses')];
    }

    protected function buildTextRequestBody(
        Provider $provider,
        string $model,
        ?string $instructions,
        array $messages,
        array $tools,
        ?array $schema,
        ?TextGenerationOptions $options,
    ): array {
        $this->activeProvider = $provider;

        try {
            $body = parent::buildTextRequestBody($provider, $model, $instructions, $messages, $tools, $schema, $options);
        } finally {
            $this->activeProvider = null;
        }

        if (($body['tools'] ?? []) === []) {
            $clientTools = $this->mapClientTools([], 'responses');
            if ($clientTools !== []) {
                $body['tools'] = $clientTools;
                $body['tool_choice'] = 'auto';
            }
        }

        return $body;
    }

    protected function mapUserMessage(UserMessage|Message $message, array &$input, ?Provider $provider = null): void
    {
        $provider ??= $this->activeProvider;

        $isEmpty = trim((string) $message->content) === ''
            && (! $message instanceof UserMessage || $message->attachments->isEmpty());

        if ($isEmpty && ! $this->hasHostScreenshot()) {
            return;
        }

        $parent = new \ReflectionMethod(parent::class, 'mapUserMessage');
        if ($parent->getNumberOfParameters() >= 3) {
            parent::mapUserMessage($message, $input, $provider);
        } else {
            parent::mapUserMessage($message, $input);
        }

        $this->appendResponsesScreenshot($input, $provider);
    }
}
