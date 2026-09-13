<?php

namespace Veda\Laravel\Gateway;

use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Gateway\DeepSeek\DeepSeekGateway;
use Laravel\Ai\Gateway\TextGenerationOptions;
use Laravel\Ai\Messages\Message;
use Laravel\Ai\Messages\UserMessage;
use Laravel\Ai\Providers\Provider;
use Laravel\Ai\Tools\ToolNameResolver;
use Veda\Laravel\Gateway\Concerns\AttachesHostScreenshot;
use Veda\Laravel\Gateway\Concerns\MapsActiveTools;
use Veda\Laravel\Gateway\Concerns\MapsClientTools;

class VedaDeepSeekGateway extends DeepSeekGateway
{
    use AttachesHostScreenshot;
    use MapsActiveTools;
    use MapsClientTools;

    protected function mapTools(array $tools, Provider $provider): array
    {
        $mapped = [];
        $activeTools = $this->activeToolNames();

        foreach ($tools as $tool) {
            if ($tool instanceof Tool && $this->shouldMapTool($tool, $activeTools)) {
                $mapped[] = $this->mapTool($tool);
            }
        }

        return [...$mapped, ...$this->mapClientTools($mapped, 'chat')];
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
        $body = parent::buildTextRequestBody($provider, $model, $instructions, $messages, $tools, $schema, $options);

        if (isset($body['messages']) && is_array($body['messages'])) {
            $body['messages'] = $this->attachHostScreenshot($body['messages'], $provider->name(), $model);
        }

        return $body;
    }

    protected function mapUserMessage(UserMessage|Message $message, array &$chatMessages): void
    {
        $isEmpty = trim((string) $message->content) === ''
            && (! $message instanceof UserMessage || $message->attachments->isEmpty());

        if ($isEmpty) {
            return;
        }

        parent::mapUserMessage($message, $chatMessages);
    }

    protected function findTool(string $name, array $tools): ?Tool
    {
        $tool = parent::findTool($name, $tools);
        if ($tool !== null) {
            return $tool;
        }

        $normalized = $this->normalizeToolName($name);
        if ($normalized === $name) {
            return null;
        }

        foreach ($tools as $candidate) {
            if (! $candidate instanceof Tool) {
                continue;
            }

            if ($this->normalizeToolName(ToolNameResolver::resolve($candidate)) === $normalized) {
                return $candidate;
            }
        }

        return null;
    }
}
