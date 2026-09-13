<?php

namespace Veda\Laravel\Tests\Unit;

use Veda\Laravel\Services\MessageContent;
use Veda\Laravel\Services\UserMessageComposer;
use Veda\Laravel\Tests\TestCase;

class UserMessageComposerTest extends TestCase
{
    public function test_compose_wraps_context_blocks_and_user_request(): void
    {
        $composer = new UserMessageComposer;

        $rendered = $composer->compose('Create course', ['LMS surface: course.'], false);

        $this->assertStringContainsString('<context>', $rendered);
        $this->assertStringContainsString('LMS surface: course.', $rendered);
        $this->assertStringContainsString('</context>', $rendered);
        $this->assertStringContainsString('<userRequest>', $rendered);
        $this->assertStringContainsString('Create course', $rendered);
    }

    public function test_compose_is_identical_on_repeat(): void
    {
        $composer = new UserMessageComposer;
        $blocks = ['LMS surface: course (entity_id: 1).'];

        $first = $composer->compose('Создай курс', $blocks);
        $second = $composer->compose('Создай курс', $blocks);

        $this->assertSame($first, $second);
    }

    public function test_compose_without_context_blocks_omits_context_tag(): void
    {
        $composer = new UserMessageComposer;

        $rendered = $composer->compose('Hello', [], false);

        $this->assertStringNotContainsString('<context>', $rendered);
        $this->assertStringContainsString('<userRequest>', $rendered);
    }

    public function test_system_initiated_messages_are_not_wrapped(): void
    {
        $composer = new UserMessageComposer;

        $this->assertSame('Raw system prompt', $composer->compose('Raw system prompt', ['ignored'], true));
    }

    public function test_extract_user_request_reads_tagged_content(): void
    {
        $composer = new UserMessageComposer;

        $rendered = $composer->compose('Show my courses', ['LMS surface: dashboard.'], false);

        $this->assertSame('Show my courses', $composer->extractUserRequest($rendered));
        $this->assertSame('plain text', $composer->extractUserRequest('plain text'));
    }

    public function test_message_content_to_text_extracts_text_parts_from_multimodal_content(): void
    {
        $content = [
            [
                'type' => 'text',
                'text' => '<userRequest>подсвети поля в шапке</userRequest>',
            ],
            [
                'type' => 'image_url',
                'image_url' => ['url' => 'data:image/jpeg;base64,abc'],
            ],
        ];

        $text = MessageContent::toText($content);

        $this->assertStringContainsString('подсвети поля в шапке', $text);
        $this->assertStringContainsString('[image]', $text);
    }
}
