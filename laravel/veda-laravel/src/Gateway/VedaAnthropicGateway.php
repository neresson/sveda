<?php

namespace Veda\Laravel\Gateway;

use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\Anthropic\AnthropicGateway;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Messages\Message;
use Laravel\Ai\Messages\UserMessage;
use Laravel\Ai\Providers\Provider;
use Veda\Laravel\Gateway\Concerns\AttachesHostScreenshot;
use Veda\Laravel\Gateway\Concerns\InvokesToolsByName;
use Veda\Laravel\Gateway\Concerns\MapsActiveTools;
use Veda\Laravel\Gateway\Concerns\MapsClientTools;

class VedaAnthropicGateway extends AnthropicGateway
{
    use AttachesHostScreenshot;
    use InvokesToolsByName;
    use MapsActiveTools;
    use MapsClientTools;

    protected ?Provider $activeProvider = null;

    protected function mapTools(array $tools, Provider $provider): array
    {
        $mapped = [];
        $activeTools = $this->activeToolNames();

        foreach ($tools as $tool) {
            if ($tool instanceof Tool && $this->shouldMapTool($tool, $activeTools)) {
                $mapped[] = $this->mapTool($tool);
            }
        }

        return [...$mapped, ...$this->mapClientTools($mapped, 'anthropic')];
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
            $clientTools = $this->mapClientTools([], 'anthropic');
            if ($clientTools !== []) {
                $body['tools'] = $clientTools;
                $body['tool_choice'] = ['type' => 'auto'];
            }
        }

        return $body;
    }

    protected function mapUserMessage(UserMessage|Message $message, array &$mapped): void
    {
        $isEmpty = trim((string) $message->content) === ''
            && (! $message instanceof UserMessage || $message->attachments->isEmpty());

        if ($isEmpty && ! $this->hasHostScreenshot()) {
            return;
        }

        parent::mapUserMessage($message, $mapped);

        $this->appendAnthropicScreenshot($mapped, $this->activeProvider);
    }
}
