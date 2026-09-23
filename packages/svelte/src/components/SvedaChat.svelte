<script lang="ts">
  import {
    History,
    Maximize2,
    MessageCircle,
    MessageSquare,
    Minimize2,
    PanelRight,
    Plus,
    Send,
    Sparkles,
    Square,
    X,
  } from 'lucide-svelte';
  import { onMount, tick } from 'svelte';
  import ChatHistoryList from './ChatHistoryList.svelte';
  import { useSvedaChatLayout } from '../hooks/useSvedaChatLayout.svelte.js';
  import {
    buildChatTabTitleMap,
    getUserMessageText,
    renderMarkdown,
    userMessageHasVisibleContent,
  } from '@sveda-ai/chat';
  import type { SvedaDisplayMessage } from '@sveda-ai/core';
  import { useSvedaConfig, useSvedaContext, useSvedaT } from '../context.js';
  import { useSvedaLauncher } from '../appearance.js';
  import { useSvedaChat } from '../hooks/useSvedaChat.svelte.js';
  import { useSvedaStreaming } from '../hooks/useSvedaStreaming.svelte.js';
  import { cn } from '../lib/utils.js';
  import type { SvedaModelOption, SvedaQuickPrompt } from '../provider.js';

  interface Props {
    models?: SvedaModelOption[];
    quickPrompts?: SvedaQuickPrompt[];
    brandName?: string;
    brandLogo?: string;
    pageUrl?: string;
    onNavigate?: (url: string) => void;
    notify?: (kind: 'error', message: string) => void;
    agentTasksSubscribe?: (onPayload: (payload: unknown) => void) => void | (() => void);
  }

  let {
    models,
    quickPrompts,
    brandName,
    brandLogo,
    pageUrl = '',
    onNavigate,
    notify,
    agentTasksSubscribe,
  }: Props = $props();

  const config = useSvedaConfig();
  const ctx = useSvedaContext();
  const t = useSvedaT();
  const launcher = useSvedaLauncher();
  const chat = useSvedaChat();
  const layout = useSvedaChatLayout({
    getIsMinimized: () => chat.isMinimized,
    hostEmbed: ctx.hostEmbed,
    fillHost: ctx.fillHost,
  });

  let showHistorySidebar = $state(false);
  let pendingDeleteChatId = $state<string | null>(null);
  let pendingRenameChatId = $state<string | null>(null);
  let renameTitle = $state('');
  const iconButtonClass =
    'inline-flex h-9 w-9 items-center justify-center bg-transparent text-muted-foreground hover:text-foreground';

  const resolvedModels = $derived(models ?? config.models);
  const resolvedPrompts = $derived(quickPrompts ?? config.quickPrompts);
  const resolvedBrandName = $derived(brandName ?? config.brand.name);
  const resolvedBrandLogo = $derived(brandLogo ?? config.brand.logoUrl ?? '');

  let inputMessage = $state('');
  let selectedChatModel = $state('');
  let messagesContainer: HTMLDivElement | null = $state(null);
  let errorBanner = $state('');

  const PAGE_SESSION_BOOT_KEY = '__SVEDA_CHAT_PAGE_BOOTED__';

  const scrollToBottom = (options?: { behavior?: string; onlyIfNearBottom?: boolean }) => {
    const el = messagesContainer;
    if (!el) return;
    if (options?.onlyIfNearBottom) {
      const distance = el.scrollHeight - el.scrollTop - el.clientHeight;
      if (distance > 120) return;
    }
    el.scrollTo({
      top: el.scrollHeight,
      behavior: (options?.behavior as ScrollBehavior) ?? 'smooth',
    });
  };

  const streaming = useSvedaStreaming(
    {
      getCurrentChat: () => chat.currentChat,
      setChatMessages: chat.setChatMessages,
      setChatTitle: chat.setChatTitle,
      refreshChatTitleFromServer: chat.refreshChatTitleFromServer,
      incrementChatTokens: chat.incrementChatTokens,
      setChatContextWindowTokens: chat.setChatContextWindowTokens,
    },
    scrollToBottom,
    {
      newChatLabel: () => t('newChat'),
      resolveSendOptions: () => {
        const model = selectedChatModel || resolvedModels[0]?.id;
        return model ? { model } : {};
      },
      onError: (_chatId, error) => {
        const message =
          error instanceof Error && error.message.trim()
            ? error.message
            : t('errorSendingMessage');
        errorBanner = message;
        notify?.('error', message);
      },
    },
  );

  const launcherLabel = $derived.by(() => {
    const custom = launcher.label.trim();
    return custom !== '' ? custom : resolvedBrandName;
  });

  const launcherImage = $derived.by(() => {
    const uploaded = launcher.image.trim();
    if (uploaded !== '') return uploaded;
    return resolvedBrandLogo;
  });

  const displayMessages = $derived(chat.currentChat?.messages ?? []);
  const hasUserMessages = $derived(
    displayMessages.some((m) => m.role === 'user' && userMessageHasVisibleContent(m)),
  );

  const tabTitles = $derived(buildChatTabTitleMap(chat.openChatTabs, t));

  const welcomeText = $derived(t('welcomeMessage'));
  const headerTitle = $derived(chat.currentChat?.title || t('chatTitle'));

  const canSend = $derived(
    inputMessage.trim().length > 0 && !chat.isLoading && !streaming.isStreaming,
  );

  const assistantText = (message: SvedaDisplayMessage): string => {
    const content = (message as { content?: unknown }).content;
    if (typeof content === 'string') return content;
    if (!Array.isArray(message.parts)) return '';
    return message.parts
      .filter((part) => part && typeof part === 'object' && (part as { type?: string }).type === 'text')
      .map((part) => String((part as { text?: string }).text ?? ''))
      .join('');
  };

  const messageHtml = (message: SvedaDisplayMessage): string => {
    if (message.role === 'user') {
      return '';
    }
    return renderMarkdown(assistantText(message));
  };

  const handleSend = async () => {
    const text = inputMessage.trim();
    if (!text || chat.isLoading || streaming.isStreaming) return;
    errorBanner = '';
    inputMessage = '';
    chat.maximizeChat();
    chat.setLoading(true);
    try {
      await streaming.sendMessage(text, pageUrl ? { pageUrl } : undefined);
    } finally {
      chat.setLoading(false);
      await tick();
      scrollToBottom({ behavior: 'smooth', onlyIfNearBottom: false });
    }
  };

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      void handleSend();
    }
  };

  const handleNewChat = () => {
    chat.createNewChat();
    inputMessage = '';
    errorBanner = '';
    if (layout.isMobile) {
      showHistorySidebar = false;
    }
  };

  const handleSelectChat = async (chatId: string) => {
    streaming.clearChatUnread(chatId);
    await chat.setCurrentChat(chatId);
    if (layout.isMobile) {
      showHistorySidebar = false;
    }
    await tick();
    scrollToBottom({ behavior: 'auto', onlyIfNearBottom: false });
  };

  const confirmDelete = () => {
    if (pendingDeleteChatId) {
      void chat.deleteChat(pendingDeleteChatId);
    }
    pendingDeleteChatId = null;
  };

  const confirmRename = () => {
    const title = renameTitle.trim();
    if (pendingRenameChatId && title) {
      chat.renameChat(pendingRenameChatId, title);
    }
    pendingRenameChatId = null;
    renameTitle = '';
  };

  const historyAsideClass = $derived(
    cn(
      'sveda-chat-surface flex min-h-0 w-80 shrink-0 flex-col overflow-hidden bg-background',
      layout.isMobile
        ? 'h-full max-w-[88vw] border-r border-border'
        : ctx.fillHost
          ? 'h-full border-r border-border'
          : layout.viewMode === 'fixed'
            ? 'absolute right-full top-0 z-10 h-full border-r border-border'
            : 'h-full rounded-[var(--sveda-radius)] border border-border',
    ),
  );

  const nonImmersiveShellClass = $derived(
    layout.isMobile || layout.viewMode === 'fixed'
      ? 'relative flex h-full min-h-0 min-w-0 flex-1 flex-row'
      : ctx.fillHost
        ? 'relative flex h-full min-h-0 min-w-0 w-full flex-row items-stretch overflow-hidden'
        : 'relative flex min-h-0 flex-row items-stretch gap-2',
  );

  const cardClass = $derived(
    cn(
      'sveda-chat-surface relative flex min-h-0 flex-col overflow-visible',
      layout.viewMode === 'floating' && !layout.isMobile && !ctx.fillHost && 'sveda-chat-frame',
      (ctx.fillHost || layout.isMobile) && 'border-0',
      layout.isMobile && 'h-full min-w-0 flex-1 rounded-none',
      layout.viewMode === 'fixed' &&
        !layout.isMobile &&
        'h-full min-w-0 flex-1 rounded-none border-b-0 border-l border-r-0 border-t-0 border-border',
      layout.viewMode === 'floating' && 'rounded-[var(--sveda-radius)]',
      (layout.isMobile || layout.viewMode === 'fixed') && 'rounded-none',
      layout.viewMode === 'floating' && !layout.isMobile && !ctx.fillHost && 'min-w-0 shrink-0',
      layout.viewMode === 'floating' && !layout.isMobile && ctx.fillHost && 'h-full min-w-0 flex-1',
    ),
  );

  const handleQuickPrompt = (prompt: SvedaQuickPrompt) => {
    inputMessage = prompt.prompt;
    void handleSend();
  };

  onMount(() => {
    if (resolvedModels[0]?.id) {
      selectedChatModel = resolvedModels[0].id;
    }

    if (typeof agentTasksSubscribe === 'function') {
      agentTasksSubscribe(() => {});
    }

    void (async () => {
      await chat.loadHistories();
      const globalScope = window as unknown as Record<string, unknown>;
      const isPageBooted = Boolean(globalScope[PAGE_SESSION_BOOT_KEY]);
      if (!isPageBooted) {
        globalScope[PAGE_SESSION_BOOT_KEY] = true;
        chat.createNewChat();
      } else if (!chat.currentChat) {
        if (chat.sortedHistories.length > 0) {
          await chat.setCurrentChat(chat.sortedHistories[0].id);
        } else {
          chat.createNewChat();
        }
      }
      await tick();
      scrollToBottom({ behavior: 'auto', onlyIfNearBottom: false });
    })();
  });
</script>

{#snippet toolbar(immersiveDesktop: boolean)}
  <header
    class="flex h-12 shrink-0 flex-row items-center justify-between border-b border-border bg-card px-3 {layout.viewMode === 'floating' && !layout.isMobile ? 'rounded-t-[var(--sveda-radius)]' : 'rounded-none'}"
  >
    {#if !immersiveDesktop}
      <div class="flex min-w-0 flex-1 items-center gap-2">
        <button
          type="button"
          class="{iconButtonClass} {showHistorySidebar ? 'bg-muted/60 text-foreground' : ''}"
          aria-label={t('chatHistory')}
          title={t('chatHistory')}
          onclick={() => (showHistorySidebar = !showHistorySidebar)}
        >
          <History class="h-4 w-4" />
        </button>
        <h2 class="min-w-0 flex-1 truncate text-left font-mono text-[11px] font-normal uppercase tracking-[0.14em] text-foreground">
          {headerTitle}
        </h2>
      </div>
    {:else}
      <div class="flex min-w-0 flex-1 items-center gap-4">
        <div class="w-[min(20rem,33vw)] min-w-0">
          <div class="flex items-center gap-2">
            {#if resolvedBrandLogo}
              <img src={resolvedBrandLogo} alt={resolvedBrandName} class="size-7 shrink-0 rounded" />
            {:else}
              <Sparkles class="size-5 shrink-0 text-primary" />
            {/if}
            <span class="truncate text-lg font-bold leading-none text-foreground">{resolvedBrandName}</span>
          </div>
        </div>
        <h2 class="min-w-0 flex-1 truncate text-left font-mono text-[11px] font-normal uppercase tracking-[0.14em] text-foreground">
          {headerTitle}
        </h2>
      </div>
    {/if}
    <div class="flex shrink-0 items-center gap-0.5">
      {#if !layout.isMobile && layout.viewMode === 'immersive'}
        <button type="button" class={iconButtonClass} aria-label={t('immersiveModeExit')} title={t('immersiveModeExit')} onclick={layout.exitImmersiveMode}>
          <Minimize2 class="h-4 w-4" />
        </button>
      {/if}
      {#if !layout.isMobile && layout.viewMode !== 'immersive'}
        <button type="button" class={iconButtonClass} aria-label={t('immersiveModeEnter')} title={t('immersiveModeEnter')} onclick={layout.enterImmersiveMode}>
          <Maximize2 class="h-4 w-4" />
        </button>
        <button
          type="button"
          class={iconButtonClass}
          aria-label={layout.viewMode === 'floating' ? t('viewModeFixed') : t('viewModeFloating')}
          title={layout.viewMode === 'floating' ? t('viewModeFixed') : t('viewModeFloating')}
          onclick={layout.toggleViewMode}
        >
          {#if layout.viewMode === 'floating'}
            <PanelRight class="h-4 w-4" />
          {:else}
            <MessageSquare class="h-4 w-4" />
          {/if}
        </button>
      {/if}
      <button type="button" class={iconButtonClass} aria-label={t('toolbarMinimize')} title={t('toolbarMinimize')} onclick={() => chat.minimizeChat()}>
        <X class="h-4 w-4" />
      </button>
    </div>
  </header>
{/snippet}

{#snippet tabs(compact: boolean)}
  {#if chat.openChatTabs.length > 0}
    <div class="flex shrink-0 items-stretch border-b border-border bg-card {compact ? 'px-3' : 'px-4'}">
      <div class="min-w-0 flex-1 overflow-x-auto">
        <div class="flex h-12 min-w-max items-end gap-px px-1">
          {#each chat.openChatTabs as tab (tab.id)}
            <div
              role="button"
              tabindex="0"
              class={cn(
                'inline-flex h-10 max-w-56 shrink-0 items-center gap-1.5 whitespace-nowrap border-b border-transparent px-3 py-2 font-mono text-[11px] tracking-[0.12em] text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none',
                chat.currentChat?.id === tab.id ? 'border-foreground bg-transparent text-foreground' : '',
              )}
              onclick={() => void handleSelectChat(tab.id)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  void handleSelectChat(tab.id);
                }
              }}
            >
              <span class="min-w-0 truncate">{tabTitles.get(tab.id) ?? tab.title}</span>
              {#if streaming.streamingChatIds.has(tab.id)}
                <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-foreground" aria-label={t('chatTabRunning')}></span>
              {:else if streaming.unreadChatIds.has(tab.id)}
                <span class="h-1.5 w-1.5 shrink-0 rounded-full bg-primary" aria-label={t('chatTabUnread')}></span>
              {/if}
              <button
                type="button"
                class="ml-0.5 inline-flex h-4 w-4 items-center justify-center text-muted-foreground hover:text-foreground"
                aria-label={t('closeChatTab')}
                onclick={(e) => {
                  e.stopPropagation();
                  void chat.closeChatTab(tab.id);
                }}
              >
                <X class="h-3 w-3" />
              </button>
            </div>
          {/each}
          <button
            type="button"
            class="inline-flex h-10 shrink-0 items-center px-2 text-muted-foreground hover:text-foreground"
            aria-label={t('newChat')}
            onclick={handleNewChat}
          >
            <Plus class="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  {/if}
{/snippet}

{#snippet thread()}
  <div bind:this={messagesContainer} class="flex min-h-0 flex-1 flex-col overflow-y-auto px-4 py-4">
    {#if !hasUserMessages}
      <div class="mt-auto flex flex-col gap-3">
        <p class="font-sans text-sm text-foreground">{welcomeText}</p>
        {#if resolvedPrompts.length > 0}
          <div class="flex flex-col gap-2">
            <p class="font-mono text-[11px] tracking-[0.08em] text-muted-foreground">{t('quickPromptsTitle')}</p>
            <div class="flex flex-wrap gap-2">
              {#each resolvedPrompts as prompt (prompt.label)}
                <button
                  type="button"
                  class="border border-border bg-secondary px-3 py-1.5 font-mono text-[11px] tracking-[0.08em] text-secondary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
                  disabled={chat.isLoading || streaming.isStreaming}
                  onclick={() => handleQuickPrompt(prompt)}
                >
                  {prompt.label}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="mt-auto flex flex-col gap-4">
        {#each displayMessages as message (message.id)}
          {#if message.role === 'user' && userMessageHasVisibleContent(message)}
            <div class="ml-8 max-w-[90%] border border-border bg-muted px-3 py-2 font-sans text-sm text-foreground">
              {getUserMessageText(message)}
            </div>
          {:else if message.role === 'assistant'}
            {@const html = messageHtml(message)}
            {#if html}
              <div class="prose mr-4 max-w-none font-sans text-sm text-foreground">
                {@html html}
              </div>
            {/if}
          {/if}
        {/each}
        {#if streaming.isThinking || streaming.isStreaming}
          <p class="font-mono text-[11px] tracking-[0.08em] text-muted-foreground">
            {streaming.thinkingMessage || t('preparingReply')}
          </p>
        {/if}
      </div>
    {/if}
  </div>
  {#if errorBanner}
    <div class="mx-3 mb-2 border border-dashed border-destructive px-3 py-2 font-mono text-[11px] tracking-[0.08em] text-destructive">
      {errorBanner}
    </div>
  {/if}
  <div class="flex flex-shrink-0 flex-col bg-background p-3">
    <div class="relative flex flex-col border border-dashed border-foreground bg-background">
      <textarea
        class="sveda-chat-input-textarea max-h-[250px] min-h-[60px] w-full resize-none border-0 bg-transparent p-3 font-sans text-sm text-foreground shadow-none outline-none ring-0 placeholder:font-mono placeholder:text-[11px] placeholder:tracking-[0.08em] placeholder:text-muted-foreground"
        style="height: 60px; outline: none; box-shadow: none"
        placeholder={t('typeMessage')}
        bind:value={inputMessage}
        disabled={chat.isLoading}
        onkeydown={handleKeydown}
      ></textarea>
      <div class="flex items-center justify-between gap-2 border-t border-border/40 px-2 py-1.5">
        <div class="min-w-0 flex-1">
          {#if resolvedModels.length > 0}
            <select
              class="max-w-full border-0 bg-transparent font-mono text-[11px] tracking-[0.08em] text-muted-foreground outline-none"
              bind:value={selectedChatModel}
              disabled={chat.isLoading || streaming.isStreaming}
            >
              {#each resolvedModels as model (model.id)}
                <option value={model.id}>{model.label || model.id}</option>
              {/each}
            </select>
          {/if}
        </div>
        {#if streaming.isStreaming}
          <button type="button" class="inline-flex h-9 w-9 items-center justify-center text-muted-foreground hover:text-foreground" aria-label="Stop" onclick={() => streaming.stopStreaming()}>
            <Square class="h-4 w-4" />
          </button>
        {:else}
          <button
            type="button"
            class="inline-flex h-9 w-9 items-center justify-center text-muted-foreground hover:text-foreground disabled:opacity-40"
            aria-label="Send"
            disabled={!canSend}
            onclick={() => void handleSend()}
          >
            <Send class="h-4 w-4" />
          </button>
        {/if}
      </div>
    </div>
  </div>
{/snippet}

{#snippet newChatButton()}
  <button
    type="button"
    class="inline-flex h-10 w-full items-center justify-center gap-2 bg-primary px-3 font-mono text-[11px] tracking-[0.14em] text-primary-foreground"
    onclick={handleNewChat}
  >
    <Plus class="h-4 w-4" />
    {t('newChat')}
  </button>
{/snippet}

{#snippet historyColumn()}
  <div class="border-b border-border/50 p-3">{@render newChatButton()}</div>
  <div class="min-h-0 flex-1">
    <ChatHistoryList
      histories={chat.sortedHistories}
      currentChatId={chat.currentChat?.id}
      onSelectChat={(chatId) => void handleSelectChat(chatId)}
      onDeleteChat={(chatId) => (pendingDeleteChatId = chatId)}
      onRenameChat={(chatId) => {
        const history = chat.sortedHistories.find((item) => item.id === chatId);
        pendingRenameChatId = chatId;
        renameTitle = history?.title ?? '';
      }}
    />
  </div>
{/snippet}

<div class={layout.chatShellClass} style={layout.chatShellStyle} data-testid="sveda-chat">
  {#if !chat.isMinimized}
    <div
      class={layout.isMobile || layout.viewMode === 'fixed' || ctx.fillHost
        ? 'flex h-full min-h-0 w-full min-w-0 flex-1'
        : layout.viewMode === 'immersive'
          ? 'flex h-full min-h-0 w-full min-w-0 flex-1 flex-col'
          : 'flex gap-2'}
    >
      {#if layout.isImmersiveDesktop}
        <div class="sveda-chat-surface flex h-full min-h-0 w-full flex-1 flex-col">
          {@render toolbar(true)}
          <div class="flex min-h-0 flex-1 flex-row">
            <aside class="sveda-chat-surface flex w-[min(20rem,33vw)] shrink-0 flex-col border-r border-border">
              {@render historyColumn()}
            </aside>
            <div class="sveda-chat-surface flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
              {@render tabs(false)}
              <div class="mx-auto flex min-h-0 w-full max-w-7xl flex-1 flex-col overflow-hidden px-4">
                {@render thread()}
              </div>
            </div>
          </div>
        </div>
      {:else}
        <div class={nonImmersiveShellClass} style={layout.floatShellStyle}>
          {#if showHistorySidebar && !layout.isMobile}
            <aside class={historyAsideClass}>
              {@render historyColumn()}
            </aside>
          {/if}
          <div class={cardClass} style={layout.chatCardStyle}>
            <div class="sveda-chat-surface relative flex min-h-0 flex-1 flex-col overflow-hidden p-0">
              <div class="flex min-h-0 flex-1 flex-col">
                {@render toolbar(false)}
                {@render tabs(true)}
                {@render thread()}
              </div>
            </div>
            {#if layout.viewMode === 'floating' && !layout.isMobile && !ctx.fillHost}
              <span class="pointer-events-none absolute -left-px -top-px z-10 h-2.5 w-2.5 border-l border-t border-foreground" aria-hidden="true"></span>
              <span class="pointer-events-none absolute -right-px -top-px z-10 h-2.5 w-2.5 border-r border-t border-foreground" aria-hidden="true"></span>
              <span class="pointer-events-none absolute -bottom-px -left-px z-10 h-2.5 w-2.5 border-b border-l border-foreground" aria-hidden="true"></span>
              <span class="pointer-events-none absolute -bottom-px -right-px z-10 h-2.5 w-2.5 border-b border-r border-foreground" aria-hidden="true"></span>
            {/if}
            {#if !layout.isMobile && layout.viewMode === 'fixed'}
              <div
                class="absolute bottom-3 left-0 top-3 z-10 w-1 touch-none cursor-ew-resize hover:bg-primary/30"
                role="separator"
                aria-orientation="vertical"
                aria-label="Resize chat"
                onpointerdown={(event) => layout.startResize('fixed-left', event)}
              ></div>
            {/if}
            {#if !layout.isMobile && layout.viewMode === 'floating'}
              <div
                class="absolute left-0 top-0 z-10 h-3 w-3 touch-none cursor-nw-resize hover:bg-primary/20"
                role="separator"
                aria-label="Resize chat"
                onpointerdown={(event) => layout.startResize('top-left', event)}
              ></div>
              <div
                class="absolute left-3 right-3 top-0 z-10 h-1 touch-none cursor-n-resize hover:bg-primary/30"
                role="separator"
                aria-orientation="horizontal"
                aria-label="Resize chat height"
                onpointerdown={(event) => layout.startResize('top', event)}
              ></div>
              <div
                class="absolute bottom-3 left-0 top-3 z-10 w-1 touch-none cursor-w-resize hover:bg-primary/30"
                role="separator"
                aria-orientation="vertical"
                aria-label="Resize chat width"
                onpointerdown={(event) => layout.startResize('left', event)}
              ></div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if layout.isMobile && showHistorySidebar && !chat.isMinimized}
    <div class="fixed inset-0 z-[60] flex bg-black/40" onclick={() => (showHistorySidebar = false)} role="presentation">
      <aside class="sveda-chat-surface flex h-full w-80 max-w-[88vw] flex-col bg-background" onclick={(event) => event.stopPropagation()} role="presentation">
        {@render historyColumn()}
      </aside>
    </div>
  {/if}

  {#if chat.isMinimized && !layout.isMobile && !ctx.hideLauncher}
    <button
      type="button"
      class="inline-flex h-auto min-w-0 items-center justify-center gap-2 rounded-none border border-primary bg-primary px-3.5 py-2 font-mono text-[11px] tracking-[0.14em] text-primary-foreground shadow-none transition-opacity hover:opacity-90"
      onclick={() => void chat.openChat()}
    >
      {#if launcherImage}
        <img src={launcherImage} alt={t('assistantLogoAlt')} class="h-6 w-6 flex-shrink-0 object-cover min-[1872px]:h-10 min-[1872px]:w-10" />
      {:else}
        <MessageCircle class="h-6 w-6 flex-shrink-0" />
      {/if}
      {#if launcherLabel}
        <span class="font-mono text-[11px] font-normal tracking-[0.14em]">{launcherLabel}</span>
      {/if}
    </button>
  {/if}

  {#if pendingDeleteChatId}
    <div class="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 p-4" role="dialog" aria-modal="true">
      <div class="w-full max-w-sm border border-border bg-card p-4 text-card-foreground shadow-lg">
        <h3 class="text-base font-semibold">{t('commonDelete')}</h3>
        <p class="mt-2 text-sm text-muted-foreground">{t('confirmDeleteChat')}</p>
        <div class="mt-4 flex justify-end gap-2">
          <button type="button" class="border border-border px-3 py-2 text-sm" onclick={() => (pendingDeleteChatId = null)}>{t('commonCancel')}</button>
          <button type="button" class="bg-primary px-3 py-2 text-sm text-primary-foreground" onclick={confirmDelete}>{t('commonDelete')}</button>
        </div>
      </div>
    </div>
  {/if}

  {#if pendingRenameChatId}
    <div class="fixed inset-0 z-[70] flex items-center justify-center bg-black/40 p-4" role="dialog" aria-modal="true">
      <div class="w-full max-w-sm border border-border bg-card p-4 text-card-foreground shadow-lg">
        <h3 class="text-base font-semibold">{t('renameChat')}</h3>
        <input
          bind:value={renameTitle}
          type="text"
          placeholder={t('renameChatPlaceholder')}
          class="mt-3 h-10 w-full border border-input bg-input px-3 text-sm outline-none"
          onkeydown={(event) => {
            if (event.key === 'Enter') confirmRename();
          }}
        />
        <div class="mt-4 flex justify-end gap-2">
          <button
            type="button"
            class="border border-border px-3 py-2 text-sm"
            onclick={() => {
              pendingRenameChatId = null;
              renameTitle = '';
            }}
          >
            {t('commonCancel')}
          </button>
          <button type="button" class="bg-primary px-3 py-2 text-sm text-primary-foreground" onclick={confirmRename}>{t('commonSave')}</button>
        </div>
      </div>
    </div>
  {/if}
</div>
