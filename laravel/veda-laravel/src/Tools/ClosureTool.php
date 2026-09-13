<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\Auth\Authenticatable;
use Illuminate\Contracts\JsonSchema\JsonSchema;
use Stringable;
use Veda\Laravel\Enums\ToolMode;

class ClosureTool extends VedaTool
{
    /**
     * @param  array<string, mixed>|\Closure  $schema
     */
    public function __construct(
        protected string $toolName,
        protected string $toolDescription,
        protected array|\Closure $toolSchema,
        protected mixed $handler,
        protected ToolMode $toolMode = ToolMode::Read,
        protected string $toolDomain = 'other',
        ?Authenticatable $user = null,
    ) {
        parent::__construct($user);
    }

    public function name(): string
    {
        return $this->toolName;
    }

    public function description(): Stringable|string
    {
        return $this->toolDescription;
    }

    public function mode(): ToolMode
    {
        return $this->toolMode;
    }

    public function domain(): string
    {
        return $this->toolDomain;
    }

    public function schema(JsonSchema $schema): array
    {
        if ($this->toolSchema instanceof \Closure) {
            $built = call_user_func($this->toolSchema, $schema);

            return is_array($built) ? $built : [];
        }

        return (new ArraySchemaConverter)->convert($schema, $this->toolSchema);
    }

    protected function execute(array $arguments): array|string
    {
        $result = call_user_func($this->handler, $arguments, $this->user);

        return is_array($result) || is_string($result)
            ? $result
            : ['success' => true, 'data' => $result];
    }
}
