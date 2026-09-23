<script lang="ts">
  import type { SvedaChatHistory } from '@sveda-ai/chat';
  import { getUserMessageText } from '@sveda-ai/chat';
  import { Search, SquarePen, Trash2 } from 'lucide-svelte';
  import { useSvedaT } from '../context.js';

  interface Props {
    histories: SvedaChatHistory[];
    currentChatId?: string | null;
    onSelectChat: (chatId: string) => void;
    onDeleteChat: (chatId: string) => void;
    onRenameChat: (chatId: string) => void;
  }

  let { histories, currentChatId = null, onSelectChat, onDeleteChat, onRenameChat }: Props = $props();

  const t = useSvedaT();
  let searchQuery = $state('');

  const previewFor = (chat: SvedaChatHistory) => {
    if (typeof chat.preview === 'string' && chat.preview.trim()) {
      return chat.preview.trim();
    }
    const message = chat.messages?.find((item) => item.role === 'user');
    return message ? getUserMessageText(message) : '';
  };

  const dayDiff = (timestamp: number) => {
    const targetDate = new Date(Number(timestamp) || Date.now());
    const now = new Date();
    const targetStart = new Date(targetDate.getFullYear(), targetDate.getMonth(), targetDate.getDate()).getTime();
    const nowStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    return Math.max(0, Math.floor((nowStart - targetStart) / 86400000));
  };

  const groups = $derived.by(() => {
    const query = searchQuery.trim().toLowerCase();
    const filtered = query
      ? histories.filter((chat) => {
          const preview = previewFor(chat);
          return chat.title.toLowerCase().includes(query) || preview.toLowerCase().includes(query);
        })
      : histories;
    const grouped: { label: string; chats: SvedaChatHistory[] }[] = [];
    const indexByLabel = new Map<string, number>();
    for (const chat of filtered) {
      const diff = dayDiff(chat.updatedAt);
      const label =
        diff === 0 ? t('today') : diff === 1 ? t('yesterday') : diff === 2 ? t('dayBeforeYesterday') : t('daysAgo', { count: diff });
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
</script>

<div class="flex h-full min-h-0 w-full flex-col bg-background">
  <div class="shrink-0 border-b border-border p-3">
    <div class="mb-3 flex items-center justify-between">
      <p class="text-sm font-semibold">{t('chatHistory')}</p>
    </div>
    <div class="flex h-10 items-center gap-2 rounded-[var(--sveda-radius)] border border-border bg-input px-3">
      <Search class="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" />
      <input
        bind:value={searchQuery}
        type="text"
        autocomplete="off"
        placeholder={t('searchChats')}
        class="h-full min-w-0 flex-1 border-0 bg-transparent p-0 text-sm text-card-foreground outline-none placeholder:text-muted-foreground"
      />
    </div>
  </div>
  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if groups.length === 0}
      <div class="px-2 py-6 text-center text-sm text-muted-foreground">{t('noChatsFound')}</div>
    {:else}
      {#each groups as group (group.label)}
        <div class="mb-3 last:mb-1">
          <p class="px-2 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">{group.label}</p>
          {#each group.chats as item (item.id)}
            <div
              role="button"
              tabindex="0"
              class="group relative cursor-pointer px-2 py-2 transition-colors hover:bg-muted {currentChatId === item.id ? 'bg-primary/10' : ''}"
              onclick={() => onSelectChat(item.id)}
              onkeydown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  onSelectChat(item.id);
                }
              }}
            >
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium">{item.title}</p>
                  <p class="truncate text-xs text-muted-foreground">{previewFor(item)}</p>
                </div>
                <div class="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                  <button
                    type="button"
                    class="inline-flex h-6 w-6 items-center justify-center"
                    aria-label={t('renameChat')}
                    onclick={(event) => {
                      event.stopPropagation();
                      onRenameChat(item.id);
                    }}
                  >
                    <SquarePen class="h-3 w-3" />
                  </button>
                  <button
                    type="button"
                    class="inline-flex h-6 w-6 items-center justify-center"
                    aria-label={t('delete')}
                    onclick={(event) => {
                      event.stopPropagation();
                      onDeleteChat(item.id);
                    }}
                  >
                    <Trash2 class="h-3 w-3" />
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>
