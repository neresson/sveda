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
        $properties = $this->properties($definition);
        $required = $definition['required'] ?? [];
        if (! is_array($required)) {
            $required = [];
        }

        $converted = [];
        foreach ($properties as $name => $spec) {
            if (! is_string($name) || $name === '') {
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
        $declaredType = $this->declaredType($spec);

        $type = match ($declaredType) {
            'string' => $schema->string(),
            'integer' => $schema->integer(),
            'number' => $schema->number(),
            'boolean' => $schema->boolean(),
            'array' => $schema->array(),
            'object' => $schema->object($this->convert($schema, $spec)),
            default => $schema->string(),
        };

        if (isset($spec['description']) && is_string($spec['description'])) {
            $type->description($spec['description']);
        }

        if (isset($spec['enum']) && is_array($spec['enum'])) {
            $type->enum($spec['enum']);
        }

        if (($spec['nullable'] ?? false) === true || $this->isNullable($spec)) {
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

    /**
     * @param  array<string, mixed>  $definition
     * @return array<string, mixed>
     */
    protected function properties(array $definition): array
    {
        if (isset($definition['properties']) && is_array($definition['properties'])) {
            return $definition['properties'];
        }

        if ($this->looksLikeJsonSchema($definition)) {
            return [];
        }

        return $definition;
    }

    /**
     * @param  array<string, mixed>  $definition
     */
    protected function looksLikeJsonSchema(array $definition): bool
    {
        return array_key_exists('type', $definition)
            || array_key_exists('properties', $definition)
            || array_key_exists('required', $definition)
            || array_key_exists('items', $definition);
    }

    /**
     * @param  array<string, mixed>  $spec
     */
    protected function declaredType(array $spec): string
    {
        $type = $spec['type'] ?? null;
        if (is_array($type)) {
            foreach ($type as $candidate) {
                if (is_string($candidate) && $candidate !== '' && $candidate !== 'null') {
                    return $candidate;
                }
            }

            return 'string';
        }

        if (is_string($type) && $type !== '') {
            return $type;
        }

        if (isset($spec['properties']) && is_array($spec['properties'])) {
            return 'object';
        }

        if (isset($spec['items'])) {
            return 'array';
        }

        return 'string';
    }

    /**
     * @param  array<string, mixed>  $spec
     */
    protected function isNullable(array $spec): bool
    {
        $type = $spec['type'] ?? null;

        return is_array($type) && in_array('null', $type, true);
    }
}
