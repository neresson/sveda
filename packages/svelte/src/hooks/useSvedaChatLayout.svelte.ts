import {
  applyFixedLeftResize,
  applyFloatingResize,
  pointerScreenDelta,
  SVEDA_CHAT_LAYOUT_MAX_HEIGHT,
  SVEDA_CHAT_LAYOUT_MAX_WIDTH,
  SVEDA_CHAT_LAYOUT_MIN_HEIGHT,
  SVEDA_CHAT_LAYOUT_MIN_WIDTH,
} from '@sveda-ai/chat';

export type SvedaChatViewMode = 'floating' | 'fixed' | 'immersive';

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

export function useSvedaChatLayout(options: {
  getIsMinimized: () => boolean;
  hostEmbed: boolean;
  fillHost: boolean;
}) {
  const initialSize = readChatDimensions();
  let viewMode = $state<SvedaChatViewMode>(readViewMode(options.hostEmbed));
  let lastNonImmersiveViewMode = $state<SvedaChatViewMode>(options.hostEmbed ? 'floating' : 'fixed');
  let isMobile = $state(false);
  let chatWidth = $state(initialSize.width);
  let chatHeight = $state(initialSize.height);
  let fixedWidth = $state(readNumber(FIXED_WIDTH_STORAGE_KEY, DEFAULT_FIXED_WIDTH));

  $effect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
      return;
    }
    const media = window.matchMedia('(max-width: 768px)');
    const apply = () => {
      isMobile = options.hostEmbed ? false : media.matches;
    };
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  });

  $effect(() => {
    if (typeof document === 'undefined') {
      return;
    }
    const minimized = options.getIsMinimized();
    const body = document.body;
    body.classList.remove(FIXED_MODE_BODY_CLASS, IMMERSIVE_BODY_CLASS);
    document.documentElement.style.removeProperty(CHAT_WIDTH_CSS_VAR);
    body.style.removeProperty(CHAT_WIDTH_CSS_VAR);

    if (!minimized && !isMobile && viewMode === 'fixed') {
      body.classList.add(FIXED_MODE_BODY_CLASS);
      body.style.setProperty(CHAT_WIDTH_CSS_VAR, `${fixedWidth}px`);
    }
    if (!minimized && !isMobile && viewMode === 'immersive') {
      body.classList.add(IMMERSIVE_BODY_CLASS);
    }

    return () => {
      body.classList.remove(FIXED_MODE_BODY_CLASS, IMMERSIVE_BODY_CLASS);
      body.style.removeProperty(CHAT_WIDTH_CSS_VAR);
    };
  });

  const saveViewMode = (mode: SvedaChatViewMode) => {
    localStorage.setItem(VIEW_MODE_STORAGE_KEY, mode);
  };

  const toggleViewMode = () => {
    if (viewMode === 'immersive') {
      return;
    }
    const next: SvedaChatViewMode = viewMode === 'floating' ? 'fixed' : 'floating';
    viewMode = next;
    lastNonImmersiveViewMode = next;
    saveViewMode(next);
  };

  const enterImmersiveMode = () => {
    if (isMobile || viewMode === 'immersive') {
      return;
    }
    lastNonImmersiveViewMode = viewMode;
    viewMode = 'immersive';
    saveViewMode('immersive');
  };

  const exitImmersiveMode = () => {
    if (viewMode !== 'immersive') {
      return;
    }
    const next = lastNonImmersiveViewMode === 'immersive' ? 'fixed' : lastNonImmersiveViewMode;
    viewMode = next;
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
    const mode = viewMode;
    const startWidth = mode === 'fixed' ? fixedWidth : chatWidth;
    const startHeight = chatHeight;
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
        fixedWidth = result.width;
        if (result.enterImmersive) {
          lastNonImmersiveViewMode = 'fixed';
          viewMode = 'immersive';
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
        chatWidth = next.width;
        chatHeight = next.height;
      }
    };

    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);
  };

  return {
    get isMobile() {
      return isMobile;
    },
    get viewMode() {
      return viewMode;
    },
    get chatWidth() {
      return chatWidth;
    },
    get chatHeight() {
      return chatHeight;
    },
    get isImmersiveDesktop() {
      return !isMobile && !options.getIsMinimized() && viewMode === 'immersive';
    },
    get chatShellClass() {
      const minimized = options.getIsMinimized();
      return [
        'sveda-chat flex flex-col',
        options.fillHost && !minimized ? 'relative h-full min-h-0 w-full overflow-hidden' : '',
        !options.fillHost && minimized ? 'fixed bottom-0 right-0 z-50 min-[1872px]:bottom-4 min-[1872px]:right-4' : '',
        !options.fillHost && !minimized && isMobile
          ? 'sveda-chat-surface fixed inset-0 z-50 pb-[env(safe-area-inset-bottom)] pt-[env(safe-area-inset-top)]'
          : '',
        !options.fillHost && !minimized && !isMobile && viewMode === 'immersive'
          ? 'sveda-chat-surface fixed right-0 top-0 z-50 h-[100dvh] pb-[env(safe-area-inset-bottom)] pt-[env(safe-area-inset-top)]'
          : '',
        !options.fillHost && !minimized && !isMobile && viewMode === 'fixed'
          ? 'sveda-chat-surface fixed right-0 top-0 z-50 h-screen pt-[env(safe-area-inset-top)]'
          : '',
        !options.fillHost && !minimized && !isMobile && viewMode === 'floating' ? 'fixed bottom-4 right-4 z-50' : '',
        options.fillHost || isMobile || viewMode === 'fixed' || viewMode === 'immersive' ? 'items-stretch' : 'items-end gap-2',
      ]
        .filter(Boolean)
        .join(' ');
    },
    get chatShellStyle() {
      const minimized = options.getIsMinimized();
      if (options.fillHost && !minimized) {
        return 'width: 100%; height: 100%;';
      }
      if (!isMobile && !minimized && viewMode === 'immersive') {
        return 'width: 100vw; height: 100dvh;';
      }
      if (!isMobile && !minimized && viewMode === 'fixed') {
        return 'width: var(--sveda-chat-width); height: 100dvh;';
      }
      return undefined;
    },
    get chatCardStyle() {
      if (isMobile || options.fillHost) {
        return 'width: 100%; height: 100%;';
      }
      if (viewMode === 'fixed') {
        return 'width: var(--sveda-chat-width); height: 100%;';
      }
      if (viewMode === 'floating') {
        return `width: ${chatWidth}px; height: ${chatHeight}px;`;
      }
      return 'width: 100%; height: 100%;';
    },
    get floatShellStyle() {
      if (options.fillHost) {
        return 'height: 100%; min-height: 0;';
      }
      if (isMobile || viewMode !== 'floating') {
        return undefined;
      }
      return `height: ${chatHeight}px; min-height: 0;`;
    },
    toggleViewMode,
    enterImmersiveMode,
    exitImmersiveMode,
    startResize,
  };
}
