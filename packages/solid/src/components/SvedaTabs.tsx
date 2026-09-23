import { buildChatTabTitleMap, type SvedaTabChat } from '@sveda-ai/chat';
import { Loader2, Plus, X } from 'lucide-solid';
import { For, Show, type Component, createMemo } from 'solid-js';
import { useSvedaT } from '../i18n/index';
import { cn } from '../lib/utils';

export interface SvedaTabsProps {
  tabs: SvedaTabChat[];
  currentChatId?: string | null;
  streamingChatIds?: Set<string>;
  unreadChatIds?: Set<string>;
  compact?: boolean;
  onSelectChat: (chatId: string) => void;
  onCloseTab: (chatId: string) => void;
  onNewChat: () => void;
}

export const SvedaTabs: Component<SvedaTabsProps> = props => {
  const t = useSvedaT();

  const visibleTabs = createMemo(() => (props.tabs ?? []).filter(tab => tab?.id));
  const tabTitles = createMemo(() => buildChatTabTitleMap(visibleTabs(), t));

  const isStreaming = (chatId: string) =>
    props.streamingChatIds instanceof Set ? props.streamingChatIds.has(chatId) : false;

  const hasUnread = (chatId: string) =>
    !isStreaming(chatId) &&
    (props.unreadChatIds instanceof Set ? props.unreadChatIds.has(chatId) : false);

  const tabTriggerClass = (chatId: string) =>
    cn(
      'inline-flex h-10 max-w-56 shrink-0 items-center gap-1.5 whitespace-nowrap border-b border-transparent px-3 py-2 font-mono text-[11px] tracking-[0.12em] text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none',
      props.currentChatId === chatId ? 'border-foreground bg-transparent text-foreground' : '',
    );

  return (
    <Show when={visibleTabs().length > 0}>
      <div
        class={cn(
          'flex shrink-0 items-stretch border-b border-border bg-card',
          props.compact ? 'px-3' : 'px-4 md:px-6',
        )}
      >
        <div class="min-w-0 flex-1 overflow-x-auto">
          <div class="flex h-12 min-w-max items-end gap-px px-1">
            <For each={visibleTabs()}>
              {tab => (
                <div
                  role="button"
                  tabindex="0"
                  class={tabTriggerClass(tab.id)}
                  onClick={() => props.onSelectChat(tab.id)}
                  onKeyDown={event => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      props.onSelectChat(tab.id);
                    }
                  }}
                >
                  <Show
                    when={isStreaming(tab.id)}
                    fallback={
                      <Show when={hasUnread(tab.id)}>
                        <span
                          class="h-2 w-2 shrink-0 rounded-full bg-primary"
                          aria-label={t('chatTabUnread')}
                        />
                      </Show>
                    }
                  >
                    <Loader2
                      class="h-3.5 w-3.5 shrink-0 animate-spin text-primary"
                      aria-label={t('chatTabRunning')}
                    />
                  </Show>
                  <span class="min-w-0 truncate">{tabTitles().get(tab.id)}</span>
                  <button
                    type="button"
                    class="ml-0.5 rounded-sm p-0.5 text-muted-foreground opacity-70 hover:bg-background/80 hover:text-foreground hover:opacity-100"
                    aria-label={t('closeChatTab')}
                    onMouseDown={event => event.stopPropagation()}
                    onClick={event => {
                      event.stopPropagation();
                      props.onCloseTab(tab.id);
                    }}
                  >
                    <X class="h-3 w-3" />
                  </button>
                </div>
              )}
            </For>
          </div>
        </div>

        <div class="flex shrink-0 items-center border-l border-border/40 pl-1">
          <button
            type="button"
            class="inline-flex h-9 w-9 shrink-0 items-center justify-center text-muted-foreground hover:text-foreground"
            aria-label={t('newChat')}
            onClick={() => props.onNewChat()}
          >
            <Plus class="h-4 w-4" />
          </button>
        </div>
      </div>
    </Show>
  );
};
