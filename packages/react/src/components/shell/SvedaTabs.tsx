import { buildChatTabTitleMap, type SvedaTabChat } from '@sveda-ai/chat';
import { Loader2, Plus, X } from 'lucide-react';
import { useMemo } from 'react';
import { useSvedaT } from '../../provider';
import { cn } from '../../lib/utils';

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

export function SvedaTabs({
  tabs,
  currentChatId = null,
  streamingChatIds = new Set(),
  unreadChatIds = new Set(),
  compact = false,
  onSelectChat,
  onCloseTab,
  onNewChat,
}: SvedaTabsProps) {
  const t = useSvedaT();
  const visibleTabs = useMemo(() => tabs.filter((tab) => tab?.id), [tabs]);
  const tabTitles = useMemo(() => buildChatTabTitleMap(visibleTabs, t), [visibleTabs, t]);

  if (visibleTabs.length === 0) {
    return null;
  }

  const isStreaming = (chatId: string) => streamingChatIds.has(chatId);
  const hasUnread = (chatId: string) => !isStreaming(chatId) && unreadChatIds.has(chatId);

  return (
    <div
      className={cn(
        'flex shrink-0 items-stretch border-b border-border bg-card',
        compact ? 'px-3' : 'px-4 md:px-6'
      )}
    >
      <div className="min-w-0 flex-1 overflow-x-auto">
        <div className="flex h-12 min-w-max items-end gap-px px-1">
          {visibleTabs.map((tab) => (
            <div
              key={tab.id}
              role="button"
              tabIndex={0}
              className={cn(
                'inline-flex h-10 max-w-56 shrink-0 items-center gap-1.5 whitespace-nowrap border-b border-transparent px-3 py-2 font-mono text-[11px] tracking-[0.12em] text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none',
                currentChatId === tab.id ? 'border-foreground bg-transparent text-foreground' : ''
              )}
              onClick={() => onSelectChat(tab.id)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  onSelectChat(tab.id);
                }
              }}
            >
              {isStreaming(tab.id) ? (
                <Loader2
                  className="h-3.5 w-3.5 shrink-0 animate-spin text-primary"
                  aria-label={t('chatTabRunning')}
                />
              ) : hasUnread(tab.id) ? (
                <span
                  className="h-2 w-2 shrink-0 rounded-full bg-primary"
                  aria-label={t('chatTabUnread')}
                />
              ) : null}
              <span className="min-w-0 truncate">{tabTitles.get(tab.id)}</span>
              <button
                type="button"
                className="ml-0.5 rounded-sm p-0.5 text-muted-foreground opacity-70 hover:bg-background/80 hover:text-foreground hover:opacity-100"
                aria-label={t('closeChatTab')}
                onMouseDown={(event) => event.stopPropagation()}
                onClick={(event) => {
                  event.stopPropagation();
                  onCloseTab(tab.id);
                }}
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="flex shrink-0 items-center border-l border-border/40 pl-1">
        <button
          type="button"
          className="inline-flex h-9 w-9 shrink-0 items-center justify-center text-muted-foreground hover:text-foreground"
          aria-label={t('newChat')}
          onClick={onNewChat}
        >
          <Plus className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}
