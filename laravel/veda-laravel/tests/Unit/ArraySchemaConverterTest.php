<?php

namespace Veda\Laravel\Tests\Unit;

use Illuminate\JsonSchema\JsonSchemaTypeFactory;
use Laravel\Ai\ObjectSchema;
use Veda\Laravel\Tests\TestCase;
use Veda\Laravel\Tools\ArraySchemaConverter;

class ArraySchemaConverterTest extends TestCase
{
    public function test_converts_nested_object_array_items(): void
    {
        $converted = (new ArraySchemaConverter)->convert(new JsonSchemaTypeFactory, [
            'type' => 'object',
            'properties' => [
                'products' => [
                    'type' => 'array',
                    'description' => 'Cart products',
                    'items' => [
                        'type' => 'object',
                        'properties' => [
                            'xml_id' => ['type' => 'integer'],
                            'q' => ['type' => 'number'],
                        ],
                        'required' => ['xml_id', 'q'],
                    ],
                ],
            ],
            'required' => ['products'],
        ]);

        $schema = (new ObjectSchema($converted, strict: false))->toSchema();

        $this->assertSame('array', $schema['properties']['products']['type']);
        $this->assertSame('integer', $schema['properties']['products']['items']['properties']['xml_id']['type']);
        $this->assertSame('number', $schema['properties']['products']['items']['properties']['q']['type']);
        $this->assertContains('xml_id', $schema['properties']['products']['items']['required']);
        $this->assertContains('q', $schema['properties']['products']['items']['required']);
    }

    public function test_empty_object_schema_does_not_invent_type_property(): void
    {
        $converted = (new ArraySchemaConverter)->convert(new JsonSchemaTypeFactory, [
            'type' => 'object',
            'properties' => [],
        ]);

        $this->assertSame([], $converted);
    }
}
