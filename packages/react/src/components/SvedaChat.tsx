import {
  getSvedaLauncherSnapshot,
  subscribeSvedaAppearance,
} from '../appearance';
import type { SvedaModelOption, SvedaQuickPrompt } from '../types';
import { getSvedaChatStore } from '@sveda-ai/chat';
import { useSvedaChat } from '../hooks/useSvedaChat';
import { useSvedaChatLayout } from '../hooks/useSvedaChatLayout';
import { useSvedaStreaming } from '../hooks/useSvedaStreaming';
import { useSvedaConfig, useSvedaContext, useSvedaT } from '../provider';
import { Plus } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { ChatHistoryList } from './shell/ChatHistoryList';
import { ChatMessageList } from './shell/ChatMessageList';
import { SvedaComposer } from './shell/SvedaComposer';
import { SvedaFrameTicks } from './shell/SvedaFrameTicks';
import { SvedaMinimizedTrigger } from './shell/SvedaMinimizedTrigger';
import { SvedaResizeHandles } from './shell/SvedaResizeHandles';
import { SvedaTabs } from './shell/SvedaTabs';
import { SvedaToolbar } from './shell/SvedaToolbar';
import { cn } from '../lib/utils';

export interface SvedaChatProps {
  models?: SvedaModelOption[];
  quickPrompts?: SvedaQuickPrompt[];
  brandName?: string;
  brandLogo?: string;
  pageUrl?: string;
  onNavigate?: (url: string) => void;
  notify?: (kind: string, message: string) => void;
  agentTasksSubscribe?: (handler: (payload: unknown) => void) => () => void;
}

export function SvedaChat({
  models: _modelsProp,
  quickPrompts: _quickPromptsProp,
  brandName,
  brandLogo,
  pageUrl = '',
  notify,
}: SvedaChatProps) {
  const config = useSvedaConfig();
  const { hideLauncher, fillHost, hostEmbed } = useSvedaContext();
  const t = useSvedaT();
  const chat = useSvedaChat();
  const layout = useSvedaChatLayout(chat.isMinimized, hostEmbed, fillHost);
  const [inputMessage, setInputMessage] = useState('');
  const [showHistorySidebar, setShowHistorySidebar] = useState(false);
  const [pendingDeleteChatId, setPendingDeleteChatId] = useState<string | null>(null);
  const [pendingRenameChatId, setPendingRenameChatId] = useState<string | null>(null);
  const [renameTitle, setRenameTitle] = useState('');
  const launcher = useSyncExternalStore(
    subscribeSvedaAppearance,
    getSvedaLauncherSnapshot,
    getSvedaLauncherSnapshot
  );

  const brandDisplayName = brandName ?? config.brand.name;
  const brandLogoUrl = brandLogo ?? config.brand.logoUrl ?? '';
  const launcherLabel = launcher.label.trim() !== '' ? launcher.label : brandDisplayName;
  const launcherImage =
    launcher.image.trim() !== '' ? launcher.image : brandLogoUrl;

  const scrollToBottom = useCallback((_options?: { behavior?: string; onlyIfNearBottom?: boolean }) => {}, []);

  const chatRef = useRef(chat);
  chatRef.current = chat;

  const streamingStore = useMemo(
    () => ({
      get currentChat() {
        return chatRef.current.currentChat;
      },
      setChatMessages: (...args: Parameters<typeof chat.setChatMessages>) =>
        chatRef.current.setChatMessages(...args),
      setChatTitle: (...args: Parameters<typeof chat.setChatTitle>) =>
        chatRef.current.setChatTitle(...args),
      refreshChatTitleFromServer: (...args: Parameters<typeof chat.refreshChatTitleFromServer>) =>
        chatRef.current.refreshChatTitleFromServer(...args),
      incrementChatTokens: (...args: Parameters<typeof chat.incrementChatTokens>) =>
        chatRef.current.incrementChatTokens(...args),
      setChatContextWindowTokens: (...args: Parameters<typeof chat.setChatContextWindowTokens>) =>
        chatRef.current.setChatContextWindowTokens(...args),
    }),
    []
  );

  const streaming = useSvedaStreaming(streamingStore, scrollToBottom, {
    onError: (_chatId, error) => {
      const message = error instanceof Error ? error.message : t('errorSendingMessage');
      notify?.('error', message);
    },
  });

  useEffect(() => {
    if (chat.isMinimized || getSvedaChatStore().getState().currentChat) {
      return;
    }
    chat.createNewChat();
  }, [chat.isMinimized, chat.currentChat, chat.createNewChat]);

  const handleSend = async () => {
    const text = inputMessage.trim();
    if (!text) {
      return;
    }
    setInputMessage('');
    await streaming.sendMessage(text, pageUrl ? { pageUrl } : undefined);
  };

  const handleNewChat = () => {
    chat.createNewChat();
    if (layout.isMobile) {
      setShowHistorySidebar(false);
    }
  };

  const handleSelectChat = (chatId: string) => {
    void chat.setCurrentChat(chatId);
    if (layout.isMobile) {
      setShowHistorySidebar(false);
    }
  };

  const confirmDelete = () => {
    if (pendingDeleteChatId) {
      void chat.deleteChat(pendingDeleteChatId);
    }
    setPendingDeleteChatId(null);
  };

  const confirmRename = () => {
    const title = renameTitle.trim();
    if (pendingRenameChatId && title) {
      chat.renameChat(pendingRenameChatId, title);
    }
    setPendingRenameChatId(null);
    setRenameTitle('');
  };

  const headerTitle = chat.currentChat?.title || t('chatTitle');

  const historyAsideClass = cn(
    'sveda-chat-surface flex min-h-0 w-80 shrink-0 flex-col overflow-hidden bg-background',
    layout.isMobile
      ? 'h-full max-w-[88vw] border-r border-border'
      : fillHost
        ? 'h-full border-r border-border'
        : layout.viewMode === 'fixed'
          ? 'absolute right-full top-0 z-10 h-full border-r border-border'
          : 'h-full rounded-[var(--sveda-radius)] border border-border'
  );

  const nonImmersiveShellClass =
    layout.isMobile || layout.viewMode === 'fixed'
      ? 'relative flex h-full min-h-0 min-w-0 flex-1 flex-row'
      : fillHost
        ? 'relative flex h-full min-h-0 min-w-0 w-full flex-row items-stretch overflow-hidden'
        : 'relative flex min-h-0 flex-row items-stretch gap-2';

  const floatShellStyle =
    fillHost
      ? { height: '100%', minHeight: 0 }
      : layout.isMobile || layout.viewMode !== 'floating'
        ? undefined
        : { height: `${layout.chatHeight}px`, minHeight: 0 };

  const cardClass = cn(
    'sveda-chat-surface relative flex min-h-0 flex-col overflow-visible',
    layout.viewMode === 'floating' && !layout.isMobile && !fillHost && 'sveda-chat-frame',
    (fillHost || layout.isMobile) && 'border-0',
    layout.isMobile && 'h-full min-w-0 flex-1 rounded-none',
    layout.viewMode === 'fixed' &&
      !layout.isMobile &&
      'h-full min-w-0 flex-1 rounded-none border-b-0 border-l border-r-0 border-t-0 border-border',
    layout.viewMode === 'floating' && 'rounded-[var(--sveda-radius)]',
    (layout.isMobile || layout.viewMode === 'fixed') && 'rounded-none',
    layout.viewMode === 'floating' && !layout.isMobile && !fillHost && 'min-w-0 shrink-0',
    layout.viewMode === 'floating' && !layout.isMobile && fillHost && 'h-full min-w-0 flex-1'
  );

  const panel = (
    <>
      <SvedaToolbar
        title={headerTitle}
        isMobile={layout.isMobile}
        viewMode={layout.viewMode}
        isImmersiveDesktop={layout.isImmersiveDesktop}
        showHistorySidebar={showHistorySidebar}
        viewModeFloatingLabel={t('viewModeFloating')}
        viewModeFixedLabel={t('viewModeFixed')}
        immersiveEnterLabel={t('immersiveModeEnter')}
        immersiveExitLabel={t('immersiveModeExit')}
        immersiveBrandName={brandDisplayName}
        immersiveBrandLogo={brandLogoUrl}
        onToggleHistory={() => setShowHistorySidebar((open) => !open)}
        onEnterImmersive={layout.enterImmersiveMode}
        onExitImmersive={layout.exitImmersiveMode}
        onToggleViewMode={layout.toggleViewMode}
        onMinimize={() => chat.minimizeChat()}
      />
      <SvedaTabs
        tabs={chat.openChatTabs}
        currentChatId={chat.currentChat?.id}
        streamingChatIds={streaming.streamingChatIds}
        unreadChatIds={streaming.unreadChatIds}
        compact
        onSelectChat={handleSelectChat}
        onCloseTab={(chatId) => {
          void chat.closeChatTab(chatId);
        }}
        onNewChat={handleNewChat}
      />
      <ChatMessageList messages={chat.currentChat?.messages ?? []} />
      <SvedaComposer
        value={inputMessage}
        onChange={setInputMessage}
        onSend={() => {
          void handleSend();
        }}
        onStop={() => streaming.stopStreaming()}
        isLoading={chat.isLoading || chat.isLoadingChatHistory}
        isStreaming={streaming.isStreaming}
      />
    </>
  );

  const newChatButton = (
    <button
      type="button"
      className="inline-flex w-full items-center justify-center gap-2 bg-primary px-3 py-2 font-mono text-[11px] tracking-[0.14em] text-primary-foreground"
      onClick={handleNewChat}
    >
      <Plus className="h-4 w-4" />
      {t('newChat')}
    </button>
  );

  const historyList = (
    <ChatHistoryList
      histories={chat.sortedHistories}
      currentChatId={chat.currentChat?.id}
      onSelectChat={handleSelectChat}
      onDeleteChat={(chatId) => setPendingDeleteChatId(chatId)}
      onRenameChat={(chatId) => {
        const history = chat.sortedHistories.find((item) => item.id === chatId);
        setPendingRenameChatId(chatId);
        setRenameTitle(history?.title ?? '');
      }}
    />
  );

  return (
    <div
      className={layout.chatShellClass}
      style={layout.chatShellStyle}
      data-testid="sveda-chat"
    >
      {!chat.isMinimized ? (
        <div
          className={
            layout.isMobile || layout.viewMode === 'fixed' || fillHost
              ? 'flex h-full min-h-0 w-full min-w-0 flex-1'
              : layout.viewMode === 'immersive'
                ? 'flex h-full min-h-0 w-full min-w-0 flex-1 flex-col'
                : 'flex gap-2'
          }
        >
          {layout.isImmersiveDesktop ? (
            <div className="sveda-chat-surface flex h-full min-h-0 w-full flex-1 flex-col">
              <SvedaToolbar
                title={headerTitle}
                isMobile={layout.isMobile}
                viewMode={layout.viewMode}
                isImmersiveDesktop
                showHistorySidebar={showHistorySidebar}
                viewModeFloatingLabel={t('viewModeFloating')}
                viewModeFixedLabel={t('viewModeFixed')}
                immersiveEnterLabel={t('immersiveModeEnter')}
                immersiveExitLabel={t('immersiveModeExit')}
                immersiveBrandName={brandDisplayName}
                immersiveBrandLogo={brandLogoUrl}
                onToggleHistory={() => setShowHistorySidebar((open) => !open)}
                onEnterImmersive={layout.enterImmersiveMode}
                onExitImmersive={layout.exitImmersiveMode}
                onToggleViewMode={layout.toggleViewMode}
                onMinimize={() => chat.minimizeChat()}
              />
              <div className="flex min-h-0 flex-1 flex-row">
                <aside className="sveda-chat-surface flex w-[min(20rem,33vw)] shrink-0 flex-col border-r border-border">
                  <div className="border-b border-border/50 p-3">{newChatButton}</div>
                  <div className="min-h-0 flex-1">{historyList}</div>
                </aside>
                <div className="sveda-chat-surface flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
                  <SvedaTabs
                    tabs={chat.openChatTabs}
                    currentChatId={chat.currentChat?.id}
                    streamingChatIds={streaming.streamingChatIds}
                    unreadChatIds={streaming.unreadChatIds}
                    onSelectChat={handleSelectChat}
                    onCloseTab={(chatId) => {
                      void chat.closeChatTab(chatId);
                    }}
                    onNewChat={handleNewChat}
                  />
                  <div className="mx-auto flex min-h-0 w-full max-w-7xl flex-1 flex-col overflow-hidden px-4 md:px-6">
                    <ChatMessageList messages={chat.currentChat?.messages ?? []} />
                    <SvedaComposer
                      value={inputMessage}
                      onChange={setInputMessage}
                      onSend={() => {
                        void handleSend();
                      }}
                      onStop={() => streaming.stopStreaming()}
                      isLoading={chat.isLoading || chat.isLoadingChatHistory}
                      isStreaming={streaming.isStreaming}
                    />
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className={nonImmersiveShellClass} style={floatShellStyle}>
              {showHistorySidebar && !layout.isMobile ? (
                <aside className={historyAsideClass}>
                  <div className="border-b border-border/50 p-3">{newChatButton}</div>
                  <div className="min-h-0 flex-1">{historyList}</div>
                </aside>
              ) : null}
              <div className={cardClass} style={layout.chatCardStyle}>
                <div className="sveda-chat-surface relative flex flex-1 flex-col overflow-hidden p-0">
                  <div className="flex min-h-0 flex-1 flex-col">{panel}</div>
                </div>
                {layout.viewMode === 'floating' && !layout.isMobile && !fillHost ? <SvedaFrameTicks /> : null}
                <SvedaResizeHandles
                  isMobile={layout.isMobile}
                  viewMode={layout.viewMode}
                  onStartResize={layout.startResize}
                />
              </div>
            </div>
          )}
        </div>
      ) : null}

      {layout.isMobile && showHistorySidebar && !chat.isMinimized ? (
        <div className="fixed inset-0 z-[60] flex bg-black/40" onClick={() => setShowHistorySidebar(false)}>
          <aside
            className="sveda-chat-surface flex h-full w-80 max-w-[88vw] flex-col bg-background"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="border-b border-border/50 p-3">{newChatButton}</div>
            <div className="min-h-0 flex-1">{historyList}</div>
          </aside>
        </div>
      ) : null}

      {chat.isMinimized && !layout.isMobile && !hideLauncher ? (
        <SvedaMinimizedTrigger
          label={launcherLabel}
          icon={launcher.icon}
          logoSrc={launcherImage}
          logoAlt={t('assistantLogoAlt')}
          onOpen={() => {
            void chat.openChat();
          }}
        />
      ) : null}

      {pendingDeleteChatId ? (
        <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 p-4" role="dialog" aria-modal="true">
          <div className="w-full max-w-sm border border-border bg-card p-4 text-card-foreground shadow-lg">
            <h3 className="text-base font-semibold">{t('commonDelete')}</h3>
            <p className="mt-2 text-sm text-muted-foreground">{t('confirmDeleteChat')}</p>
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                className="border border-border px-3 py-2 text-sm"
                onClick={() => setPendingDeleteChatId(null)}
              >
                {t('commonCancel')}
              </button>
              <button type="button" className="bg-primary px-3 py-2 text-sm text-primary-foreground" onClick={confirmDelete}>
                {t('commonDelete')}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {pendingRenameChatId ? (
        <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 p-4" role="dialog" aria-modal="true">
          <div className="w-full max-w-sm border border-border bg-card p-4 text-card-foreground shadow-lg">
            <h3 className="text-base font-semibold">{t('renameChat')}</h3>
            <input
              value={renameTitle}
              placeholder={t('renameChatPlaceholder')}
              className="mt-3 h-10 w-full border border-input bg-input px-3 text-sm outline-none"
              onChange={(event) => setRenameTitle(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  confirmRename();
                }
              }}
            />
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                className="border border-border px-3 py-2 text-sm"
                onClick={() => {
                  setPendingRenameChatId(null);
                  setRenameTitle('');
                }}
              >
                {t('commonCancel')}
              </button>
              <button type="button" className="bg-primary px-3 py-2 text-sm text-primary-foreground" onClick={confirmRename}>
                {t('commonSave')}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

export default SvedaChat;
