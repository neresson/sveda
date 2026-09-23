import {
  applyFixedLeftResize,
  applyFloatingResize,
  hasPendingToolConfirmation,
  pointerScreenDelta,
  SVEDA_CHAT_LAYOUT_MAX_HEIGHT,
  SVEDA_CHAT_LAYOUT_MAX_WIDTH,
  SVEDA_CHAT_LAYOUT_MIN_HEIGHT,
  SVEDA_CHAT_LAYOUT_MIN_WIDTH,
  userMessageHasVisibleContent,
  type SvedaUserMessageLike,
} from '@sveda-ai/chat';
import type { SvedaSendOptions } from '@sveda-ai/core';
import {
  createEffect,
  createMemo,
  createSignal,
  onMount,
  type Accessor,
} from 'solid-js';
import { useSvedaChrome, useSvedaLauncher } from '../appearance';
import { useSvedaT } from '../i18n/index';
import { useSvedaConfig, useSvedaContext, type SvedaModelOption, type SvedaQuickPrompt } from '../provider';
import { useSvedaChat } from './useSvedaChat';
import { useSvedaStreaming } from './useSvedaStreaming';

const SVEDA_CHAT_MODEL_STORAGE_KEY = 'sveda.chat-model';
const SVEDA_CHAT_THINKING_STORAGE_KEY = 'sveda.chat-thinking';
const PAGE_SESSION_BOOT_KEY = '__SVEDA_CHAT_PAGE_BOOTED__';
const VIEW_MODE_STORAGE_KEY = 'sveda.chat-view-mode';
const FIXED_WIDTH_STORAGE_KEY = 'sveda.chat-fixed-width';
const CHAT_STORAGE_KEY = 'sveda.chat-dimensions';
const FIXED_MODE_BODY_CLASS = 'sveda-chat-fixed-mode';
const IMMERSIVE_BODY_CLASS = 'sveda-chat-immersive-mode';
const CHAT_WIDTH_CSS_VAR = '--sveda-chat-width';
const FIXED_RESIZE_BODY_CLASS = 'sveda-chat-fixed-resizing';
const DEFAULT_WIDTH = 384;
const DEFAULT_HEIGHT = 600;
const DEFAULT_FIXED_WIDTH = 400;
const MAIN_CONTENT_MIN_WIDTH = 768;

type SvedaChatViewMode = 'floating' | 'fixed' | 'immersive';

const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

const readChatDimensions = (): { width: number; height: number } => {
  if (typeof localStorage === 'undefined') {
    return { width: DEFAULT_WIDTH, height: DEFAULT_HEIGHT };
  }
  try {
    const saved = localStorage.getItem(CHAT_STORAGE_KEY);
    if (!saved) {
      return { width: DEFAULT_WIDTH, height: DEFAULT_HEIGHT };
    }
    const parsed = JSON.parse(saved) as { width?: number; height?: number };
    return {
      width: clamp(Number(parsed.width) || DEFAULT_WIDTH, SVEDA_CHAT_LAYOUT_MIN_WIDTH, SVEDA_CHAT_LAYOUT_MAX_WIDTH),
      height: clamp(Number(parsed.height) || DEFAULT_HEIGHT, SVEDA_CHAT_LAYOUT_MIN_HEIGHT, SVEDA_CHAT_LAYOUT_MAX_HEIGHT),
    };
  } catch {
    return { width: DEFAULT_WIDTH, height: DEFAULT_HEIGHT };
  }
};

const readViewMode = (hostEmbed: boolean): SvedaChatViewMode => {
  if (typeof localStorage === 'undefined') {
    return hostEmbed ? 'floating' : 'fixed';
  }
  const stored = localStorage.getItem(VIEW_MODE_STORAGE_KEY);
  if (stored === 'floating' || stored === 'fixed' || stored === 'immersive') {
    return stored;
  }
  return hostEmbed ? 'floating' : 'fixed';
};

const readNumber = (key: string, fallback: number): number => {
  if (typeof localStorage === 'undefined') {
    return fallback;
  }
  const parsed = Number(localStorage.getItem(key));
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
};

export interface SvedaAgentTasksPayload {
  phase?: string;
  tasks?: Array<{ id?: string; label?: string; status?: string }>;
}

export interface SvedaChatPageOptions {
  models?: Accessor<SvedaModelOption[]>;
  quickPrompts?: Accessor<SvedaQuickPrompt[]>;
  brandName?: Accessor<string | undefined>;
  brandLogo?: Accessor<string | undefined>;
  pageUrl?: Accessor<string | undefined>;
  onNavigate?: (url: string) => void;
  notify?: (kind: 'error', message: string) => void;
  agentTasksSubscribe?: (
    onPayload: (payload: SvedaAgentTasksPayload) => void,
  ) => void | (() => void);
}

const svedaErrorMessage = (error: unknown, fallback: string): string => {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === 'string' && message.trim()) {
      return message;
    }
  }
  return fallback;
};

export function useSvedaChatPage(options: SvedaChatPageOptions = {}) {
  const t = useSvedaT();
  const config = useSvedaConfig();
  const ctx = useSvedaContext();
  const launcher = useSvedaLauncher();
  const chrome = useSvedaChrome();

  const chatStore = useSvedaChat();
  const {
    currentChat,
    sortedHistories,
    openChatTabs,
    isMinimized,
    isLoading,
    isLoadingChatHistory,
    hasCurrentChat,
    loadHistories,
    createNewChat,
    setCurrentChat,
    setChatTitle,
    refreshChatTitleFromServer,
    setChatMessages,
    incrementChatTokens,
    setChatContextWindowTokens,
    deleteChat,
    renameChat,
    closeChatTab,
    openChat,
    maximizeChat,
    minimizeChat,
    setLoading,
  } = chatStore;

  const [inputMessage, setInputMessage] = createSignal('');
  const [isMobile, setIsMobile] = createSignal(
    typeof window !== 'undefined' ? window.matchMedia('(max-width: 768px)').matches : false,
  );
  const initialSize = readChatDimensions();
  const [chatWidth, setChatWidth] = createSignal(initialSize.width);
  const [chatHeight, setChatHeight] = createSignal(initialSize.height);
  const [fixedWidth, setFixedWidth] = createSignal(readNumber(FIXED_WIDTH_STORAGE_KEY, DEFAULT_FIXED_WIDTH));
  const [viewMode, setViewMode] = createSignal<SvedaChatViewMode>(readViewMode(ctx.hostEmbed));
  const [lastNonImmersiveViewMode, setLastNonImmersiveViewMode] = createSignal<'floating' | 'fixed'>(
    ctx.hostEmbed ? 'floating' : 'fixed',
  );
  const [showHistorySidebar, setShowHistorySidebar] = createSignal(false);

  const models = createMemo(() => options.models?.() ?? config.models);
  const quickPrompts = createMemo(() => options.quickPrompts?.() ?? config.quickPrompts);

  const readStoredModel = (): string => {
    if (typeof window === 'undefined') {
      return models()[0]?.id ?? '';
    }
    const saved = localStorage.getItem(SVEDA_CHAT_MODEL_STORAGE_KEY);
    if (saved && models().some(option => option.id === saved)) {
      return saved;
    }
    return models()[0]?.id ?? '';
  };

  const [selectedChatModel, setSelectedChatModel] = createSignal(readStoredModel());
  const [thinkingEnabled, setThinkingEnabled] = createSignal(
    typeof window === 'undefined'
      ? true
      : localStorage.getItem(SVEDA_CHAT_THINKING_STORAGE_KEY) !== '0',
  );

  createEffect(() => {
    const list = models();
    if (list.length > 0 && !list.some(option => option.id === selectedChatModel())) {
      setSelectedChatModel(list[0].id);
    }
  });

  createEffect(() => {
    const value = selectedChatModel();
    if (typeof window !== 'undefined' && value) {
      localStorage.setItem(SVEDA_CHAT_MODEL_STORAGE_KEY, value);
    }
  });

  createEffect(() => {
    if (typeof window !== 'undefined') {
      localStorage.setItem(SVEDA_CHAT_THINKING_STORAGE_KEY, thinkingEnabled() ? '1' : '0');
    }
  });

  onMount(() => {
    const mq = window.matchMedia('(max-width: 768px)');
    const onChange = () => setIsMobile(mq.matches && !ctx.hostEmbed);
    onChange();
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  createEffect(() => {
    if (typeof document === 'undefined') {
      return;
    }
    const body = document.body;
    body.classList.remove(FIXED_MODE_BODY_CLASS, IMMERSIVE_BODY_CLASS);
    document.documentElement.style.removeProperty(CHAT_WIDTH_CSS_VAR);
    body.style.removeProperty(CHAT_WIDTH_CSS_VAR);
    if (!isMinimized() && !isMobile() && viewMode() === 'fixed') {
      body.classList.add(FIXED_MODE_BODY_CLASS);
      body.style.setProperty(CHAT_WIDTH_CSS_VAR, `${fixedWidth()}px`);
    }
    if (!isMinimized() && !isMobile() && viewMode() === 'immersive') {
      body.classList.add(IMMERSIVE_BODY_CLASS);
    }
  });

  const saveViewMode = (mode: SvedaChatViewMode) => {
    localStorage.setItem(VIEW_MODE_STORAGE_KEY, mode);
  };

  const toggleViewMode = () => {
    if (viewMode() === 'immersive') {
      return;
    }
    const next: SvedaChatViewMode = viewMode() === 'floating' ? 'fixed' : 'floating';
    setViewMode(next);
    setLastNonImmersiveViewMode(next);
    saveViewMode(next);
  };

  const enterImmersiveMode = () => {
    if (isMobile() || viewMode() === 'immersive') {
      return;
    }
    setLastNonImmersiveViewMode(viewMode() === 'floating' ? 'floating' : 'fixed');
    setViewMode('immersive');
    saveViewMode('immersive');
  };

  const exitImmersiveMode = () => {
    if (viewMode() !== 'immersive') {
      return;
    }
    const next = lastNonImmersiveViewMode();
    setViewMode(next);
    saveViewMode(next);
  };

  const startResize = (direction: string, event: PointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    const handle = event.currentTarget;
    if (!(handle instanceof HTMLElement)) {
      return;
    }
    const pointerId = event.pointerId;
    try {
      handle.setPointerCapture(pointerId);
    } catch {
      // Capture is best-effort.
    }

    const startX = event.screenX;
    const startY = event.screenY;
    const mode = viewMode();
    const startWidth = mode === 'fixed' ? fixedWidth() : chatWidth();
    const startHeight = chatHeight();
    let latestWidth = startWidth;
    let latestHeight = startHeight;
    let finished = false;

    const stop = () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onUp);
      try {
        handle.releasePointerCapture(pointerId);
      } catch {
        // Already released.
      }
    };

    const onUp = () => {
      if (finished) {
        return;
      }
      finished = true;
      document.body.classList.remove(FIXED_RESIZE_BODY_CLASS);
      if (mode === 'floating') {
        localStorage.setItem(CHAT_STORAGE_KEY, JSON.stringify({ width: latestWidth, height: latestHeight }));
      }
      if (mode === 'fixed') {
        localStorage.setItem(FIXED_WIDTH_STORAGE_KEY, String(latestWidth));
      }
      stop();
    };

    const onMove = (moveEvent: PointerEvent) => {
      if (finished || moveEvent.pointerId !== pointerId) {
        return;
      }
      const { deltaX, deltaY } = pointerScreenDelta(moveEvent, startX, startY);
      if (mode === 'fixed' && direction === 'fixed-left') {
        document.body.classList.add(FIXED_RESIZE_BODY_CLASS);
        const maxChat = Math.max(SVEDA_CHAT_LAYOUT_MIN_WIDTH, window.innerWidth - MAIN_CONTENT_MIN_WIDTH);
        const result = applyFixedLeftResize(startWidth, deltaX, maxChat);
        latestWidth = result.width;
        setFixedWidth(result.width);
        if (result.enterImmersive) {
          setLastNonImmersiveViewMode('fixed');
          setViewMode('immersive');
          localStorage.setItem(VIEW_MODE_STORAGE_KEY, 'immersive');
          localStorage.setItem(FIXED_WIDTH_STORAGE_KEY, String(result.width));
          finished = true;
          document.body.classList.remove(FIXED_RESIZE_BODY_CLASS);
          stop();
        }
        return;
      }
      if (mode === 'floating') {
        const next = applyFloatingResize(direction, startWidth, startHeight, deltaX, deltaY);
        latestWidth = next.width;
        latestHeight = next.height;
        setChatWidth(next.width);
        setChatHeight(next.height);
      }
    };

    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);
  };

  const isImmersiveDesktop = createMemo(
    () => !isMobile() && !isMinimized() && viewMode() === 'immersive',
  );

  const scrollToBottom = (_opts?: { behavior?: string; onlyIfNearBottom?: boolean }) => {
    const el = document.querySelector('.sveda-chat [data-sveda-messages]');
    if (el instanceof HTMLElement) {
      el.scrollTop = el.scrollHeight;
    }
  };

  const resolveStreamingSendOptions = (): Pick<SvedaSendOptions, 'model' | 'options'> => {
    const supportsThinking =
      models().find(option => option.id === selectedChatModel())?.supportsThinking ?? false;
    return {
      model: selectedChatModel() || undefined,
      options: {
        thinking: Boolean(chrome.thinking && supportsThinking && thinkingEnabled()),
      },
    };
  };

  const streaming = useSvedaStreaming(
    {
      currentChat,
      setChatMessages,
      setChatTitle,
      refreshChatTitleFromServer,
      incrementChatTokens,
      setChatContextWindowTokens,
    },
    scrollToBottom,
    {
      newChatLabel: () => t('newChat'),
      resolveSendOptions: resolveStreamingSendOptions,
      onError: (_chatId, error) => {
        options.notify?.('error', svedaErrorMessage(error, t('errorSendingMessage')));
      },
    },
  );

  const messages = createMemo(() => currentChat()?.messages ?? []);
  const hasUserMessages = createMemo(() =>
    (messages() as SvedaUserMessageLike[]).some(userMessageHasVisibleContent),
  );

  const quickPromptButtons = createMemo(() => {
    const list = quickPrompts();
    if (!Array.isArray(list)) return [];
    return list
      .map(item => {
        const label = String(item?.label ?? '').trim();
        const prompt = String(item?.prompt ?? '').trim();
        return label && prompt ? { label, prompt } : null;
      })
      .filter((item): item is SvedaQuickPrompt => !!item);
  });

  const showQuickPromptButtons = createMemo(() => {
    if (!currentChat() || quickPromptButtons().length === 0) return false;
    return !hasUserMessages();
  });

  const confirmationPending = createMemo(() =>
    hasPendingToolConfirmation(currentChat()?.messages),
  );

  const brandDisplayName = createMemo(
    () => options.brandName?.() ?? config.brand.name ?? 'Sveda',
  );
  const brandLogo = createMemo(() => options.brandLogo?.() ?? config.brand.logoUrl ?? null);
  const headerTitle = createMemo(() => currentChat()?.title || t('chatTitle'));
  const launcherLabel = createMemo(() => {
    const custom = launcher.label.trim();
    return custom !== '' ? custom : brandDisplayName();
  });
  const launcherIcon = createMemo(() => launcher.icon);
  const launcherImage = createMemo(() => {
    const uploaded = launcher.image.trim();
    if (uploaded !== '') return uploaded;
    return brandLogo() || '';
  });

  const fillHost = ctx.fillHost;
  const hideLauncher = ctx.hideLauncher;

  const chatShellStyle = createMemo(() => {
    if (fillHost && !isMinimized()) {
      return { width: '100%', height: '100%' };
    }
    if (!isMobile() && !isMinimized() && viewMode() === 'immersive') {
      return { width: '100vw', height: '100dvh' };
    }
    if (!isMobile() && !isMinimized() && viewMode() === 'fixed') {
      return { width: 'var(--sveda-chat-width)', height: '100dvh' };
    }
    return undefined;
  });

  const chatCardStyle = createMemo(() => {
    if (isMobile() || fillHost) {
      return { width: '100%', height: '100%' };
    }
    if (viewMode() === 'fixed') {
      return { width: 'var(--sveda-chat-width)', height: '100%' };
    }
    if (viewMode() === 'floating') {
      return { width: `${chatWidth()}px`, height: `${chatHeight()}px` };
    }
    return { width: '100%', height: '100%' };
  });

  const floatShellStyle = createMemo(() => {
    if (fillHost) {
      return { height: '100%', minHeight: 0 };
    }
    if (isMobile() || viewMode() !== 'floating') {
      return undefined;
    }
    return { height: `${chatHeight()}px`, minHeight: 0 };
  });

  const sendMessage = async () => {
    if (isLoading() || confirmationPending() || !currentChat()) return;
    const text = inputMessage().trim();
    if (!text) return;

    try {
      setLoading(true);
      await ctx.beforeSend?.();
    } catch {
      return;
    } finally {
      setLoading(false);
    }

    maximizeChat();
    setInputMessage('');
    await streaming.sendMessage(
      { displayText: text, promptText: text },
      {},
    );
    scrollToBottom({ behavior: 'smooth', onlyIfNearBottom: false });
  };

  const sendQuickPrompt = async (prompt: string) => {
    if (!prompt || isLoading() || confirmationPending() || !currentChat()) return;
    setInputMessage(prompt);
    await sendMessage();
  };

  const handleNewChat = () => {
    createNewChat();
  };

  const handleSelectChat = async (chatId: string) => {
    streaming.clearChatUnread(chatId);
    await setCurrentChat(chatId);
    scrollToBottom({ behavior: 'auto', onlyIfNearBottom: false });
  };

  onMount(async () => {
    await loadHistories();
    const globalScope = window as unknown as Record<string, unknown>;
    const isPageBooted = Boolean(globalScope[PAGE_SESSION_BOOT_KEY]);
    if (!isPageBooted) {
      globalScope[PAGE_SESSION_BOOT_KEY] = true;
      createNewChat();
    } else if (!currentChat()) {
      if (sortedHistories().length > 0) {
        await setCurrentChat(sortedHistories()[0].id);
      } else {
        createNewChat();
      }
    }
    scrollToBottom();
  });

  const shellClass = createMemo(() => {
    const classes = ['sveda-chat', 'flex', 'flex-col'];
    if (fillHost) {
      classes.push('relative', 'h-full', 'min-h-0', 'w-full', 'overflow-hidden', 'items-stretch');
    } else if (isMinimized()) {
      classes.push(
        'fixed',
        'bottom-0',
        'right-0',
        'z-50',
        'min-[1872px]:bottom-4',
        'min-[1872px]:right-4',
        'items-end',
        'gap-2',
      );
    } else if (isMobile()) {
      classes.push(
        'sveda-chat-surface',
        'fixed',
        'inset-0',
        'z-50',
        'pb-[env(safe-area-inset-bottom)]',
        'pt-[env(safe-area-inset-top)]',
        'items-stretch',
      );
    } else if (viewMode() === 'immersive') {
      classes.push(
        'sveda-chat-surface',
        'fixed',
        'right-0',
        'top-0',
        'z-50',
        'h-[100dvh]',
        'pb-[env(safe-area-inset-bottom)]',
        'pt-[env(safe-area-inset-top)]',
        'items-stretch',
      );
    } else if (viewMode() === 'fixed') {
      classes.push(
        'sveda-chat-surface',
        'fixed',
        'right-0',
        'top-0',
        'z-50',
        'h-screen',
        'pt-[env(safe-area-inset-top)]',
        'items-stretch',
      );
    } else {
      classes.push('fixed', 'bottom-4', 'right-4', 'z-50', 'items-end', 'gap-2');
    }
    return classes.join(' ');
  });

  return {
    t,
    currentChat,
    sortedHistories,
    openChatTabs,
    isMinimized,
    isLoading,
    isLoadingChatHistory,
    hasCurrentChat,
    isMobile,
    viewMode,
    setViewMode,
    showHistorySidebar,
    setShowHistorySidebar,
    isImmersiveDesktop,
    fixedWidth,
    toggleViewMode,
    enterImmersiveMode,
    exitImmersiveMode,
    startResize,
    fillHost,
    hideLauncher,
    inputMessage,
    setInputMessage,
    selectedChatModel,
    setSelectedChatModel,
    thinkingEnabled,
    setThinkingEnabled,
    chatModels: models,
    messages,
    hasUserMessages,
    quickPromptButtons,
    showQuickPromptButtons,
    confirmationPending,
    brandDisplayName,
    brandLogo,
    headerTitle,
    launcherLabel,
    launcherIcon,
    launcherImage,
    chatShellStyle,
    chatCardStyle,
    floatShellStyle,
    shellClass,
    chatWidth,
    chatHeight,
    thinkingTooltipText: createMemo(() => t('thinkingTooltip')),
    sendMessage,
    sendQuickPrompt,
    handleNewChat,
    handleSelectChat,
    deleteChat,
    renameChat,
    closeChatTab,
    openChat,
    minimizeChat,
    maximizeChat,
    isStreaming: streaming.isStreaming,
    isThinking: streaming.isThinking,
    thinkingMessage: streaming.thinkingMessage,
    stopStreaming: streaming.stopStreaming,
    streamingChatIds: streaming.streamingChatIds,
    unreadChatIds: streaming.unreadChatIds,
    resolveToolConfirmation: streaming.resolveToolConfirmation,
    pageUrl: createMemo(() => options.pageUrl?.() ?? ''),
    onNavigate: options.onNavigate,
  };
}
