import {
  getUserMessageText,
  messageHasVisibleAssistantContent,
  renderMarkdown,
  type SvedaChatMessageLike,
} from '@sveda-ai/chat';
import { For, Show, type Component, createMemo } from 'solid-js';
import { useSvedaT } from '../i18n/index';
import { cn } from '../lib/utils';

export interface SvedaMessageListProps {
  messages: SvedaChatMessageLike[];
  isLoading?: boolean;
  isThinking?: boolean;
  thinkingMessage?: string;
  class?: string;
}

const assistantText = (message: SvedaChatMessageLike): string => {
  if (!Array.isArray(message.parts)) {
    return typeof message.content === 'string' ? message.content : '';
  }
  return message.parts
    .filter(part => part.type === 'text' && Boolean(part.text?.trim()))
    .map(part => String(part.text ?? ''))
    .join('\n\n');
};

export const SvedaMessageList: Component<SvedaMessageListProps> = props => {
  const t = useSvedaT();

  const visibleMessages = createMemo(() =>
    (props.messages ?? []).filter(message => {
      if (message.role === 'user') {
        return getUserMessageText(message).length > 0;
      }
      if (message.role === 'assistant') {
        return messageHasVisibleAssistantContent(message);
      }
      return false;
    }),
  );

  return (
    <div
      data-sveda-messages
      class={cn('flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-4 py-3', props.class)}
    >
      <For each={visibleMessages()}>
        {message => (
          <div
            class={cn(
              'max-w-[92%] rounded-[var(--sveda-radius)] px-3 py-2 text-sm',
              message.role === 'user'
                ? 'ml-auto border border-border bg-muted text-foreground'
                : 'mr-auto border border-border/60 bg-background text-foreground',
            )}
          >
            <Show
              when={message.role === 'user'}
              fallback={
                <div
                  class="sveda-markdown prose prose-sm max-w-none dark:prose-invert"
                  // eslint-disable-next-line solid/no-innerhtml
                  innerHTML={renderMarkdown(assistantText(message))}
                />
              }
            >
              <p class="whitespace-pre-wrap">{getUserMessageText(message)}</p>
            </Show>
          </div>
        )}
      </For>

      <Show when={props.isThinking}>
        <div class="mr-auto flex items-center gap-2 px-1 font-mono text-[11px] tracking-[0.08em] text-muted-foreground">
          <span class="inline-block h-2 w-2 animate-pulse rounded-full bg-primary" />
          {props.thinkingMessage || t('thinking') || 'Thinking…'}
        </div>
      </Show>

      <Show when={props.isLoading && !props.isThinking && visibleMessages().length === 0}>
        <div class="px-1 font-mono text-[11px] tracking-[0.08em] text-muted-foreground">
          {t('loading') || 'Loading…'}
        </div>
      </Show>
    </div>
  );
};
