<?php

namespace Veda\Laravel\Services;

use Illuminate\Support\Facades\DB;

class ConversationMessageExporter
{
    /**
     * @return array{messages: array<int, array<string, mixed>>, conversationHistory: array<int, array<string, mixed>>}
     */
    public function export(string $conversationId): array
    {
        $messagesTable = config('veda.tables.messages') ?? config('ai.conversations.tables.messages', 'agent_conversation_messages');

        $records = DB::table($messagesTable)
            ->where('conversation_id', $conversationId)
            ->orderBy('created_at')
            ->orderBy('id')
            ->get();

        if ($records->isEmpty()) {
            return [
                'messages' => [],
                'conversationHistory' => [],
            ];
        }

        $uiMessages = [];
        $conversationHistory = [];
        $pendingAssistant = null;

        foreach ($records as $record) {
            if ($record->role === 'user') {
                if (trim((string) $record->content) === '') {
                    continue;
                }

                if ($pendingAssistant !== null) {
                    $uiMessages[] = $pendingAssistant;
                    $pendingAssistant = null;
                }

                $userMessage = [
                    'id' => $record->id,
                    'role' => 'user',
                    'content' => (string) $record->content,
                    'timestamp' => $this->timestampMs($record->created_at),
                ];
                $uiMessages[] = $userMessage;
                $conversationHistory[] = [
                    'role' => 'user',
                    'content' => $userMessage['content'],
                ];

                continue;
            }

            if ($record->role !== 'assistant') {
                continue;
            }

            $parts = $this->buildPartsFromAssistantRecord($record);
            $assistantMessage = [
                'id' => $record->id,
                'role' => 'assistant',
                'content' => trim((string) $record->content),
                'parts' => $parts,
                'timestamp' => $this->timestampMs($record->created_at),
            ];

            if ($pendingAssistant === null) {
                $pendingAssistant = $assistantMessage;
            } else {
                $pendingAssistant['parts'] = array_merge(
                    $pendingAssistant['parts'] ?? [],
                    $parts
                );
                if ($assistantMessage['content'] !== '') {
                    $pendingAssistant['content'] = trim(
                        ($pendingAssistant['content'] ?? '')."\n\n".$assistantMessage['content']
                    );
                }
                $pendingAssistant['id'] = $assistantMessage['id'];
                $pendingAssistant['timestamp'] = $assistantMessage['timestamp'];
            }

            $conversationHistory[] = [
                'role' => 'assistant',
                'content' => (string) $record->content,
            ];
        }

        if ($pendingAssistant !== null) {
            $uiMessages[] = $pendingAssistant;
        }

        return [
            'messages' => $uiMessages,
            'conversationHistory' => $conversationHistory,
        ];
    }

    /**
     * @return array<int, array<string, mixed>>
     */
    protected function buildPartsFromAssistantRecord(object $record): array
    {
        $parts = [];
        $content = trim((string) $record->content);

        if ($content !== '') {
            $parts[] = [
                'type' => 'text',
                'text' => $content,
                'state' => 'done',
            ];
        }

        $toolCalls = collect(json_decode($record->tool_calls ?? '[]', true) ?: []);
        $toolResults = collect(json_decode($record->tool_results ?? '[]', true) ?: [])
            ->keyBy(fn (array $result) => (string) ($result['id'] ?? ''));

        foreach ($toolCalls as $toolCall) {
            if (! is_array($toolCall)) {
                continue;
            }

            $toolCallId = (string) ($toolCall['id'] ?? '');
            $toolName = (string) ($toolCall['name'] ?? '');
            if ($toolName === '') {
                continue;
            }

            $part = [
                'type' => 'tool-'.$toolName,
                'toolCallId' => $toolCallId,
                'toolName' => $toolName,
                'input' => $toolCall['arguments'] ?? [],
                'state' => 'output-available',
            ];

            $result = $toolResults->get($toolCallId);
            if (is_array($result)) {
                $output = $result['result'] ?? null;
                if (is_array($output) || is_object($output)) {
                    $part['output'] = json_encode($output, JSON_UNESCAPED_UNICODE);
                } elseif (is_string($output)) {
                    $part['output'] = $output;
                } elseif ($output !== null) {
                    $part['output'] = (string) $output;
                }
            }

            $parts[] = $part;
        }

        return $parts;
    }

    protected function timestampMs(mixed $value): int
    {
        if ($value === null) {
            return (int) round(microtime(true) * 1000);
        }

        return (int) (strtotime((string) $value) * 1000);
    }
}
