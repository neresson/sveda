<?php

namespace Veda\Laravel\Support;

use Laravel\Ai\Responses\Data\ToolCall;
use Laravel\Ai\Responses\Data\ToolResult;

final class FrontendToolContinuation
{
    /**
     * @param  array<int, array<string, mixed>>  $messages
     * @return array{toolCalls: array<int, ToolCall>, toolResults: array<int, ToolResult>}|null
     */
    public static function fromMessages(array $messages): ?array
    {
        if ($messages === []) {
            return null;
        }

        $lastMessage = end($messages);
        if (! is_array($lastMessage) || ($lastMessage['role'] ?? '') !== 'tool') {
            return null;
        }

        $results = self::extractToolResultParts($lastMessage);
        if ($results === []) {
            return null;
        }

        $callsById = self::extractToolCallParts($messages);

        $toolCalls = [];
        $toolResults = [];

        foreach ($results as $result) {
            $callId = $result['toolCallId'];
            $call = $callsById[$callId] ?? null;

            $arguments = is_array($call['input'] ?? null) ? $call['input'] : [];
            $name = is_string($call['toolName'] ?? null) && $call['toolName'] !== ''
                ? $call['toolName']
                : $result['toolName'];

            $toolCalls[] = new ToolCall($callId, $name, $arguments, $callId);
            $toolResults[] = new ToolResult($callId, $name, $arguments, $result['output'], $callId);
        }

        return ['toolCalls' => $toolCalls, 'toolResults' => $toolResults];
    }

    /**
     * @param  array<string, mixed>  $toolMessage
     * @return array<int, array{toolCallId: string, toolName: string, output: mixed}>
     */
    protected static function extractToolResultParts(array $toolMessage): array
    {
        $parts = $toolMessage['parts'] ?? null;
        if (! is_array($parts)) {
            return [];
        }

        $results = [];
        foreach ($parts as $part) {
            if (! is_array($part) || ($part['type'] ?? '') !== 'tool-result') {
                continue;
            }

            $toolCallId = $part['toolCallId'] ?? null;
            if (! is_string($toolCallId) || $toolCallId === '') {
                continue;
            }

            $results[] = [
                'toolCallId' => $toolCallId,
                'toolName' => is_string($part['toolName'] ?? null) ? $part['toolName'] : '',
                'output' => $part['output'] ?? null,
            ];
        }

        return $results;
    }

    /**
     * @param  array<int, array<string, mixed>>  $messages
     * @return array<string, array{toolName: string, input: mixed}>
     */
    protected static function extractToolCallParts(array $messages): array
    {
        $calls = [];

        for ($index = count($messages) - 2; $index >= 0; $index--) {
            $message = $messages[$index];
            if (! is_array($message)) {
                continue;
            }

            $role = $message['role'] ?? '';
            if ($role === 'tool') {
                continue;
            }
            if ($role !== 'assistant') {
                break;
            }

            $parts = $message['parts'] ?? null;
            if (! is_array($parts)) {
                continue;
            }

            foreach ($parts as $part) {
                if (! is_array($part) || ($part['type'] ?? '') !== 'tool-call') {
                    continue;
                }

                $toolCallId = $part['toolCallId'] ?? null;
                if (! is_string($toolCallId) || $toolCallId === '') {
                    continue;
                }

                $calls[$toolCallId] = [
                    'toolName' => is_string($part['toolName'] ?? null) ? $part['toolName'] : '',
                    'input' => $part['input'] ?? ($part['args'] ?? []),
                ];
            }

            break;
        }

        return $calls;
    }
}
