import type { SvedaDisplayMessage } from '@sveda-ai/core';
import { getUserMessageText } from '@sveda-ai/chat';
import { cn } from '../../lib/utils';

const getAssistantText = (message: SvedaDisplayMessage): string => {
  if (!Array.isArray(message.parts)) {
    return '';
  }

  return message.parts
    .filter((part): part is { type: 'text'; text: string } => part.type === 'text' && typeof (part as { text?: string }).text === 'string')
    .map((part) => part.text)
    .join('');
};

export interface ChatMessageListProps {
  messages: SvedaDisplayMessage[];
  className?: string;
}

export function ChatMessageList({ messages, className }: ChatMessageListProps) {
  return (
    <div
      className={cn('flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-4 py-3', className)}
      data-testid="sveda-message-list"
    >
      <div className="mt-auto flex flex-col gap-3">
        {messages.map((message) => {
          const isUser = message.role === 'user';
          const text = isUser ? getUserMessageText(message) : getAssistantText(message);
          if (!text.trim()) {
            return null;
          }

          return (
            <div
              key={message.id}
              className={cn(
                'max-w-[90%] whitespace-pre-wrap break-words font-sans text-sm leading-relaxed',
                isUser
                  ? 'ml-auto border border-border bg-secondary px-3 py-2 text-secondary-foreground'
                  : 'mr-auto text-foreground'
              )}
            >
              {text}
            </div>
          );
        })}
      </div>
    </div>
  );
}
