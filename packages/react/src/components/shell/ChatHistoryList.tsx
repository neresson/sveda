import type { SvedaChatHistory } from '@sveda-ai/chat';
import { getUserMessageText } from '@sveda-ai/chat';
import { Search, SquarePen, Trash2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useSvedaT } from '../../provider';
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

export function ChatHistoryList({
  histories,
  currentChatId = null,
  onSelectChat,
  onDeleteChat,
  onRenameChat,
}: ChatHistoryListProps) {
  const t = useSvedaT();
  const [searchQuery, setSearchQuery] = useState('');

  const previewFor = (chat: SvedaChatHistory) => {
    if (typeof chat.preview === 'string' && chat.preview.trim()) {
      return chat.preview.trim();
    }
    const message = chat.messages?.find((item) => item.role === 'user');
    return message ? getUserMessageText(message) : '';
  };

  const filtered = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return histories;
    }
    return histories.filter((chat) => {
      const preview = previewFor(chat);
      return chat.title.toLowerCase().includes(query) || preview.toLowerCase().includes(query);
    });
  }, [histories, searchQuery]);

  const groups = useMemo(() => {
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
  }, [filtered, t]);

  return (
    <div className="flex h-full min-h-0 w-full flex-col bg-background">
      <div className="shrink-0 border-b border-border p-3">
        <div className="mb-3 flex items-center justify-between">
          <p className="text-sm font-semibold">{t('chatHistory')}</p>
        </div>
        <div className="flex h-10 items-center gap-2 rounded-[var(--sveda-radius)] border border-border bg-input px-3">
          <Search className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <input
            value={searchQuery}
            type="text"
            autoComplete="off"
            placeholder={t('searchChats')}
            className="h-full min-w-0 flex-1 border-0 bg-transparent p-0 text-sm text-card-foreground outline-none placeholder:text-muted-foreground"
            onChange={(event) => setSearchQuery(event.target.value)}
          />
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {groups.length === 0 ? (
          <div className="px-2 py-6 text-center text-sm text-muted-foreground">{t('noChatsFound')}</div>
        ) : (
          groups.map((group) => (
            <div key={group.label} className="mb-3 last:mb-1">
              <p className="px-2 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">{group.label}</p>
              {group.chats.map((chat) => (
                <div
                  key={chat.id}
                  role="button"
                  tabIndex={0}
                  className={cn(
                    'group relative cursor-pointer px-2 py-2 transition-colors hover:bg-muted',
                    currentChatId === chat.id ? 'bg-primary/10' : ''
                  )}
                  onClick={() => onSelectChat(chat.id)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      onSelectChat(chat.id);
                    }
                  }}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium">{chat.title}</p>
                      <p className="truncate text-xs text-muted-foreground">{previewFor(chat)}</p>
                    </div>
                    <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                      <button
                        type="button"
                        className="inline-flex h-6 w-6 items-center justify-center"
                        aria-label={t('renameChat')}
                        onClick={(event) => {
                          event.stopPropagation();
                          onRenameChat(chat.id);
                        }}
                      >
                        <SquarePen className="h-3 w-3" />
                      </button>
                      <button
                        type="button"
                        className="inline-flex h-6 w-6 items-center justify-center"
                        aria-label={t('delete')}
                        onClick={(event) => {
                          event.stopPropagation();
                          onDeleteChat(chat.id);
                        }}
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
