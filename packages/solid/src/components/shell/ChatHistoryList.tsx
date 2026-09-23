import type { SvedaChatHistory } from '@sveda-ai/chat';
import { getUserMessageText } from '@sveda-ai/chat';
import { Search, SquarePen, Trash2 } from 'lucide-solid';
import { For, Show, createMemo, createSignal, type Component } from 'solid-js';
import { useSvedaT } from '../../i18n/index';
import { cn } from '../../lib/utils';

export interface ChatHistoryListProps {
  histories: SvedaChatHistory[];
  currentChatId?: string | null;
  onSelectChat: (chatId: string) => void;
  onDeleteChat: (chatId: string) => void;
  onRenameChat: (chatId: string) => void;
}

const dayDiff = (timestamp: number) => {
  const targetDate = new Date(Number(timestamp) || Date.now());
  const now = new Date();
  const targetStart = new Date(targetDate.getFullYear(), targetDate.getMonth(), targetDate.getDate()).getTime();
  const nowStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  return Math.max(0, Math.floor((nowStart - targetStart) / 86400000));
};

export const ChatHistoryList: Component<ChatHistoryListProps> = props => {
  const t = useSvedaT();
  const [searchQuery, setSearchQuery] = createSignal('');

  const previewFor = (chat: SvedaChatHistory) => {
    if (typeof chat.preview === 'string' && chat.preview.trim()) {
      return chat.preview.trim();
    }
    const message = chat.messages?.find(item => item.role === 'user');
    return message ? getUserMessageText(message) : '';
  };

  const filtered = createMemo(() => {
    const query = searchQuery().trim().toLowerCase();
    const histories = props.histories;
    if (!query) {
      return histories;
    }
    return histories.filter(chat => {
      const preview = previewFor(chat);
      return chat.title.toLowerCase().includes(query) || preview.toLowerCase().includes(query);
    });
  });

  const groups = createMemo(() => {
    const grouped: { label: string; chats: SvedaChatHistory[] }[] = [];
    const indexByLabel = new Map<string, number>();
    for (const chat of filtered()) {
      const diff = dayDiff(chat.updatedAt);
      const label =
        diff === 0
          ? t('today')
          : diff === 1
            ? t('yesterday')
            : diff === 2
              ? t('dayBeforeYesterday')
              : t('daysAgo', { count: diff });
      const index = indexByLabel.get(label);
      if (index === undefined) {
        indexByLabel.set(label, grouped.length);
        grouped.push({ label, chats: [chat] });
      } else {
        grouped[index].chats.push(chat);
      }
    }
    return grouped;
  });

  return (
    <div class="flex h-full min-h-0 w-full flex-col bg-background">
      <div class="shrink-0 border-b border-border p-3">
        <div class="mb-3 flex items-center justify-between">
          <p class="text-sm font-semibold">{t('chatHistory')}</p>
        </div>
        <div class="flex h-10 items-center gap-2 rounded-[var(--sveda-radius)] border border-border bg-input px-3">
          <Search class="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <input
            value={searchQuery()}
            type="text"
            autocomplete="off"
            placeholder={t('searchChats')}
            class="h-full min-w-0 flex-1 border-0 bg-transparent p-0 text-sm text-card-foreground outline-none placeholder:text-muted-foreground"
            onInput={event => setSearchQuery(event.currentTarget.value)}
          />
        </div>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto p-2">
        <Show
          when={groups().length > 0}
          fallback={<div class="px-2 py-6 text-center text-sm text-muted-foreground">{t('noChatsFound')}</div>}
        >
          <For each={groups()}>
            {group => (
              <div class="mb-3 last:mb-1">
                <p class="px-2 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  {group.label}
                </p>
                <For each={group.chats}>
                  {chat => (
                    <div
                      role="button"
                      tabIndex={0}
                      class={cn(
                        'group relative cursor-pointer px-2 py-2 transition-colors hover:bg-muted',
                        props.currentChatId === chat.id ? 'bg-primary/10' : '',
                      )}
                      onClick={() => props.onSelectChat(chat.id)}
                      onKeyDown={event => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          props.onSelectChat(chat.id);
                        }
                      }}
                    >
                      <div class="flex items-start justify-between gap-2">
                        <div class="min-w-0 flex-1">
                          <p class="truncate text-sm font-medium">{chat.title}</p>
                          <p class="truncate text-xs text-muted-foreground">{previewFor(chat)}</p>
                        </div>
                        <div class="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                          <button
                            type="button"
                            class="inline-flex h-6 w-6 items-center justify-center"
                            aria-label={t('renameChat')}
                            onClick={event => {
                              event.stopPropagation();
                              props.onRenameChat(chat.id);
                            }}
                          >
                            <SquarePen class="h-3 w-3" />
                          </button>
                          <button
                            type="button"
                            class="inline-flex h-6 w-6 items-center justify-center"
                            aria-label={t('delete')}
                            onClick={event => {
                              event.stopPropagation();
                              props.onDeleteChat(chat.id);
                            }}
                          >
                            <Trash2 class="h-3 w-3" />
                          </button>
                        </div>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  );
};
