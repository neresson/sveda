<?php

namespace Veda\Laravel\Tools;

use Illuminate\Contracts\JsonSchema\JsonSchema;
use Illuminate\JsonSchema\Types\ArrayType;
use Illuminate\JsonSchema\Types\Type;

class ArraySchemaConverter
{
    /**
     * @param  array<string, mixed>  $definition
     * @return array<string, Type>
     */
    public function convert(JsonSchema $schema, array $definition): array
    {
        $properties = $definition['properties'] ?? $definition;
        if (! is_array($properties)) {
            return [];
        }

        $required = $definition['required'] ?? [];
        if (! is_array($required)) {
            $required = [];
        }

        $converted = [];
        foreach ($properties as $name => $spec) {
            if (! is_string($name)) {
                continue;
            }

            $type = $this->convertProperty($schema, is_array($spec) ? $spec : []);
            if ($type === null) {
                continue;
            }

            if (in_array($name, $required, true)) {
                $type->required();
            }

            $converted[$name] = $type;
        }

        return $converted;
    }

    /**
     * @param  array<string, mixed>  $spec
     */
    protected function convertProperty(JsonSchema $schema, array $spec): ?Type
    {
        $type = match ((string) ($spec['type'] ?? 'string')) {
            'string' => $schema->string(),
            'integer' => $schema->integer(),
            'number' => $schema->number(),
            'boolean' => $schema->boolean(),
            'array' => $schema->array(),
            'object' => $schema->object(),
            default => $schema->string(),
        };

        if (isset($spec['description']) && is_string($spec['description'])) {
            $type->description($spec['description']);
        }

        if (isset($spec['enum']) && is_array($spec['enum'])) {
            $type->enum($spec['enum']);
        }

        if (($spec['nullable'] ?? false) === true) {
            $type->nullable();
        }

        if ($type instanceof ArrayType && isset($spec['items']) && is_array($spec['items'])) {
            $items = $this->convertProperty($schema, $spec['items']);
            if ($items instanceof Type) {
                $type->items($items);
            }
        }

        return $type;
    }
}
