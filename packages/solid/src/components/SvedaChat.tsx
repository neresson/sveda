import { Plus } from 'lucide-solid';
import { Show, createMemo, createSignal, type Component, type JSX } from 'solid-js';
import { useSvedaChatPage } from '../hooks/useSvedaChatPage';
import type { SvedaAgentTasksPayload } from '../hooks/useSvedaChatPage';
import { useSvedaConfig, useSvedaContext, type SvedaModelOption, type SvedaQuickPrompt } from '../provider';
import { cn } from '../lib/utils';
import { SvedaComposer } from './SvedaComposer';
import { SvedaMessageList } from './SvedaMessageList';
import { SvedaMinimizedTrigger } from './SvedaMinimizedTrigger';
import { SvedaQuickPrompts } from './SvedaQuickPrompts';
import { SvedaTabs } from './SvedaTabs';
import { SvedaToolbar } from './SvedaToolbar';
import { SvedaWelcome } from './SvedaWelcome';
import { ChatHistoryList } from './shell/ChatHistoryList';
import { SvedaFrameTicks } from './shell/SvedaFrameTicks';
import { SvedaResizeHandles } from './shell/SvedaResizeHandles';

export interface SvedaChatProps {
  models?: SvedaModelOption[];
  quickPrompts?: SvedaQuickPrompt[];
  brandName?: string;
  brandLogo?: string;
  pageUrl?: string;
  onNavigate?: (url: string) => void;
  notify?: (kind: 'error', message: string) => void;
  agentTasksSubscribe?: (
    onPayload: (payload: SvedaAgentTasksPayload) => void,
  ) => void | (() => void);
}

export const SvedaChat: Component<SvedaChatProps> = props => {
  const config = useSvedaConfig();
  const ctx = useSvedaContext();

  const chat = useSvedaChatPage({
    models: () => props.models ?? config.models,
    quickPrompts: () => props.quickPrompts ?? config.quickPrompts,
    brandName: () => props.brandName,
    brandLogo: () => props.brandLogo,
    pageUrl: () => props.pageUrl,
    onNavigate: props.onNavigate,
    notify: props.notify,
    agentTasksSubscribe: props.agentTasksSubscribe,
  });

  const [pendingDeleteChatId, setPendingDeleteChatId] = createSignal<string | null>(null);
  const [pendingRenameChatId, setPendingRenameChatId] = createSignal<string | null>(null);
  const [renameTitle, setRenameTitle] = createSignal('');

  const handleNewChat = () => {
    chat.handleNewChat();
    if (chat.isMobile()) {
      chat.setShowHistorySidebar(false);
    }
  };

  const handleSelectChat = (chatId: string) => {
    void chat.handleSelectChat(chatId);
    if (chat.isMobile()) {
      chat.setShowHistorySidebar(false);
    }
  };

  const confirmDelete = () => {
    const chatId = pendingDeleteChatId();
    if (chatId) {
      void chat.deleteChat(chatId);
    }
    setPendingDeleteChatId(null);
  };

  const confirmRename = () => {
    const chatId = pendingRenameChatId();
    const title = renameTitle().trim();
    if (chatId && title) {
      chat.renameChat(chatId, title);
    }
    setPendingRenameChatId(null);
    setRenameTitle('');
  };

  const historyAsideClass = createMemo(() =>
    cn(
      'sveda-chat-surface flex min-h-0 w-80 shrink-0 flex-col overflow-hidden bg-background',
      chat.isMobile()
        ? 'h-full max-w-[88vw] border-r border-border'
        : chat.fillHost
          ? 'h-full border-r border-border'
          : chat.viewMode() === 'fixed'
            ? 'absolute right-full top-0 z-10 h-full border-r border-border'
            : 'h-full rounded-[var(--sveda-radius)] border border-border',
    ),
  );

  const nonImmersiveShellClass = createMemo(() =>
    chat.isMobile() || chat.viewMode() === 'fixed'
      ? 'relative flex h-full min-h-0 min-w-0 flex-1 flex-row'
      : chat.fillHost
        ? 'relative flex h-full min-h-0 min-w-0 w-full flex-row items-stretch overflow-hidden'
        : 'relative flex min-h-0 flex-row items-stretch gap-2',
  );

  const cardClass = createMemo(() =>
    cn('sveda-chat-surface relative flex min-h-0 flex-col overflow-visible', {
      'sveda-chat-frame': chat.viewMode() === 'floating' && !chat.isMobile() && !chat.fillHost,
      'border-0': chat.fillHost || chat.isMobile(),
      'h-full min-w-0 flex-1 rounded-none': chat.isMobile(),
      'h-full min-w-0 flex-1 rounded-none border-b-0 border-l border-r-0 border-t-0 border-border':
        chat.viewMode() === 'fixed' && !chat.isMobile(),
      'rounded-[var(--sveda-radius)]': chat.viewMode() === 'floating',
      'rounded-none': chat.isMobile() || chat.viewMode() === 'fixed',
      'min-w-0 shrink-0': chat.viewMode() === 'floating' && !chat.isMobile() && !chat.fillHost,
      'h-full min-w-0 flex-1': chat.viewMode() === 'floating' && !chat.isMobile() && chat.fillHost,
    }),
  );

  const toolbar = (immersiveDesktop: boolean): JSX.Element => (
    <SvedaToolbar
      isMobile={chat.isMobile()}
      viewMode={chat.viewMode()}
      isImmersiveDesktop={immersiveDesktop}
      showHistorySidebar={chat.showHistorySidebar()}
      title={chat.headerTitle()}
      viewModeFloatingLabel={chat.t('viewModeFloating')}
      viewModeFixedLabel={chat.t('viewModeFixed')}
      immersiveEnterLabel={chat.t('immersiveModeEnter')}
      immersiveExitLabel={chat.t('immersiveModeExit')}
      immersiveBrandName={chat.brandDisplayName()}
      immersiveBrandLogo={chat.brandLogo() ?? undefined}
      onToggleHistorySidebar={() => chat.setShowHistorySidebar(!chat.showHistorySidebar())}
      onEnterImmersive={chat.enterImmersiveMode}
      onExitImmersive={chat.exitImmersiveMode}
      onToggleViewMode={chat.toggleViewMode}
      onMinimize={chat.minimizeChat}
    />
  );

  const thread = (compact: boolean): JSX.Element => (
    <>
      <SvedaTabs
        tabs={chat.openChatTabs()}
        currentChatId={chat.currentChat()?.id}
        streamingChatIds={chat.streamingChatIds()}
        unreadChatIds={chat.unreadChatIds()}
        compact={compact}
        onSelectChat={handleSelectChat}
        onCloseTab={id => void chat.closeChatTab(id)}
        onNewChat={handleNewChat}
      />
      <Show
        when={chat.hasUserMessages()}
        fallback={
          <SvedaWelcome
            inputMessage={chat.inputMessage()}
            onInputMessage={chat.setInputMessage}
            isLoading={chat.isLoading() || chat.isLoadingChatHistory()}
            confirmationPending={chat.confirmationPending()}
            hasCurrentChat={chat.hasCurrentChat()}
            isStreaming={chat.isStreaming()}
            showQuickPromptButtons={chat.showQuickPromptButtons()}
            quickPromptButtons={chat.quickPromptButtons()}
            quickPromptsTitle={chat.t('quickPromptsTitle')}
            onSend={() => void chat.sendMessage()}
            onStop={() => chat.stopStreaming()}
            onSendQuickPrompt={prompt => void chat.sendQuickPrompt(prompt)}
          />
        }
      >
        <SvedaMessageList
          messages={chat.messages()}
          isLoading={chat.isLoading() || chat.isLoadingChatHistory()}
          isThinking={chat.isThinking()}
          thinkingMessage={chat.thinkingMessage()}
        />
        <Show when={chat.showQuickPromptButtons()}>
          <SvedaQuickPrompts
            class="shrink-0 px-4 pb-1 pt-0"
            prompts={chat.quickPromptButtons()}
            title={chat.t('quickPromptsTitle')}
            disabled={chat.isLoading() || chat.isStreaming() || chat.confirmationPending()}
            onSelect={prompt => void chat.sendQuickPrompt(prompt)}
          />
        </Show>
        <SvedaComposer
          value={chat.inputMessage()}
          onInput={chat.setInputMessage}
          isLoading={chat.isLoading() || chat.isLoadingChatHistory()}
          confirmationPending={chat.confirmationPending()}
          hasCurrentChat={chat.hasCurrentChat()}
          isStreaming={chat.isStreaming()}
          onSend={() => void chat.sendMessage()}
          onStop={() => chat.stopStreaming()}
        />
      </Show>
    </>
  );

  const newChatButton = (): JSX.Element => (
    <button
      type="button"
      class="inline-flex w-full items-center justify-center gap-2 bg-primary px-3 py-2 font-mono text-[11px] tracking-[0.14em] text-primary-foreground"
      onClick={handleNewChat}
    >
      <Plus class="h-4 w-4" />
      {chat.t('newChat')}
    </button>
  );

  const historyColumn = (): JSX.Element => (
    <>
      <div class="border-b border-border/50 p-3">{newChatButton()}</div>
      <div class="min-h-0 flex-1">
        <ChatHistoryList
          histories={chat.sortedHistories()}
          currentChatId={chat.currentChat()?.id}
          onSelectChat={handleSelectChat}
          onDeleteChat={chatId => setPendingDeleteChatId(chatId)}
          onRenameChat={chatId => {
            const history = chat.sortedHistories().find(item => item.id === chatId);
            setPendingRenameChatId(chatId);
            setRenameTitle(history?.title ?? '');
          }}
        />
      </div>
    </>
  );

  return (
    <div class={chat.shellClass()} style={chat.chatShellStyle()}>
      <Show when={!chat.isMinimized()}>
        <div
          class={
            chat.isMobile() || chat.viewMode() === 'fixed' || chat.fillHost
              ? 'flex h-full min-h-0 w-full min-w-0 flex-1'
              : chat.viewMode() === 'immersive'
                ? 'flex h-full min-h-0 w-full min-w-0 flex-1 flex-col'
                : 'flex gap-2'
          }
        >
          <Show
            when={chat.isImmersiveDesktop()}
            fallback={
              <div class={nonImmersiveShellClass()} style={chat.floatShellStyle()}>
                <Show when={chat.showHistorySidebar() && !chat.isMobile()}>
                  <aside class={historyAsideClass()}>{historyColumn()}</aside>
                </Show>
                <div class={cardClass()} style={chat.chatCardStyle()}>
                  <div class="sveda-chat-surface relative flex flex-1 flex-col overflow-hidden p-0">
                    <div class="flex min-h-0 flex-1 flex-col">
                      {toolbar(false)}
                      {thread(true)}
                    </div>
                  </div>
                  <Show when={chat.viewMode() === 'floating' && !chat.isMobile() && !chat.fillHost}>
                    <SvedaFrameTicks />
                  </Show>
                  <SvedaResizeHandles
                    isMobile={chat.isMobile()}
                    viewMode={chat.viewMode()}
                    onStartResize={chat.startResize}
                  />
                </div>
              </div>
            }
          >
            <div class="sveda-chat-surface flex h-full min-h-0 w-full flex-1 flex-col">
              {toolbar(true)}
              <div class="flex min-h-0 flex-1 flex-row">
                <aside class="sveda-chat-surface flex w-[min(20rem,33vw)] shrink-0 flex-col border-r border-border">
                  {historyColumn()}
                </aside>
                <div class="sveda-chat-surface flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
                  <div class="mx-auto flex min-h-0 w-full max-w-7xl flex-1 flex-col overflow-hidden px-4 md:px-6">
                    {thread(false)}
                  </div>
                </div>
              </div>
            </div>
          </Show>
        </div>
      </Show>

      <Show when={chat.isMobile() && chat.showHistorySidebar() && !chat.isMinimized()}>
        <div
          class="fixed inset-0 z-[60] flex bg-black/40"
          onClick={() => chat.setShowHistorySidebar(false)}
        >
          <aside
            class="sveda-chat-surface flex h-full w-80 max-w-[88vw] flex-col bg-background"
            onClick={event => event.stopPropagation()}
          >
            {historyColumn()}
          </aside>
        </div>
      </Show>

      <Show when={chat.isMinimized() && !chat.isMobile() && !ctx.hideLauncher}>
        <SvedaMinimizedTrigger
          label={chat.launcherLabel()}
          icon={chat.launcherIcon()}
          logoSrc={chat.launcherImage()}
          logoAlt={chat.t('assistantLogoAlt')}
          onOpen={() => void chat.openChat()}
        />
      </Show>

      <Show when={pendingDeleteChatId()}>
        <div
          class="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 p-4"
          role="dialog"
          aria-modal="true"
        >
          <div class="w-full max-w-sm border border-border bg-card p-4 text-card-foreground shadow-lg">
            <h3 class="text-base font-semibold">{chat.t('commonDelete')}</h3>
            <p class="mt-2 text-sm text-muted-foreground">{chat.t('confirmDeleteChat')}</p>
            <div class="mt-4 flex justify-end gap-2">
              <button
                type="button"
                class="border border-border px-3 py-2 text-sm"
                onClick={() => setPendingDeleteChatId(null)}
              >
                {chat.t('commonCancel')}
              </button>
              <button
                type="button"
                class="bg-primary px-3 py-2 text-sm text-primary-foreground"
                onClick={confirmDelete}
              >
                {chat.t('commonDelete')}
              </button>
            </div>
          </div>
        </div>
      </Show>

      <Show when={pendingRenameChatId()}>
        <div
          class="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 p-4"
          role="dialog"
          aria-modal="true"
        >
          <div class="w-full max-w-sm border border-border bg-card p-4 text-card-foreground shadow-lg">
            <h3 class="text-base font-semibold">{chat.t('renameChat')}</h3>
            <input
              value={renameTitle()}
              placeholder={chat.t('renameChatPlaceholder')}
              class="mt-3 h-10 w-full border border-input bg-input px-3 text-sm outline-none"
              onInput={event => setRenameTitle(event.currentTarget.value)}
              onKeyDown={event => {
                if (event.key === 'Enter') {
                  confirmRename();
                }
              }}
            />
            <div class="mt-4 flex justify-end gap-2">
              <button
                type="button"
                class="border border-border px-3 py-2 text-sm"
                onClick={() => {
                  setPendingRenameChatId(null);
                  setRenameTitle('');
                }}
              >
                {chat.t('commonCancel')}
              </button>
              <button
                type="button"
                class="bg-primary px-3 py-2 text-sm text-primary-foreground"
                onClick={confirmRename}
              >
                {chat.t('commonSave')}
              </button>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default SvedaChat;
