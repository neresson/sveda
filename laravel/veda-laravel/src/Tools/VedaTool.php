<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\Auth\Authenticatable;
use Laravel\Ai\Contracts\Tool;
use Laravel\Ai\Tools\Request;
use Laravel\Ai\Tools\ToolNameResolver;
use Stringable;
use Veda\Laravel\Enums\ToolMode;
use Veda\Laravel\VedaManager;

abstract class VedaTool implements Tool
{
    public function __construct(
        protected ?Authenticatable $user = null,
    ) {}

    abstract public function name(): string;

    abstract public function description(): Stringable|string;

    /**
     * @return array<string, mixed>|string
     */
    abstract protected function execute(array $arguments): array|string;

    public function mode(): ToolMode
    {
        return ToolMode::Read;
    }

    public function domain(): string
    {
        return 'other';
    }

    public function availableInSession(): bool
    {
        return true;
    }

    public function allowedDuringEmbedWriteFilter(): bool
    {
        return false;
    }

    public function handle(Request $request): Stringable|string
    {
        $toolName = ToolNameResolver::resolve($this);

        try {
            $arguments = app(ToolArgumentNormalizer::class)->normalize($toolName, $request->all());

            $result = $this->execute($arguments);

            if (is_array($result)) {
                $normalized = [
                    'success' => $result['success'] ?? true,
                ];

                if (isset($result['data'])) {
                    $normalized['data'] = $result['data'];
                } elseif (isset($result['error'])) {
                    $normalized['error'] = $result['error'];
                    $normalized['success'] = false;
                } else {
                    $normalized['data'] = $result;
                }

                $normalized = app(VedaManager::class)->enrichToolResult($toolName, $arguments, $normalized);

                return json_encode($normalized, JSON_UNESCAPED_UNICODE);
            }

            return $result;
        } catch (\Throwable $e) {
            return json_encode([
                'success' => false,
                'error' => $e->getMessage(),
            ], JSON_UNESCAPED_UNICODE);
        }
    }
}
