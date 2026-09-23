import {
  applyFixedLeftResize,
  applyFloatingResize,
  pointerScreenDelta,
  SVEDA_CHAT_LAYOUT_MAX_HEIGHT,
  SVEDA_CHAT_LAYOUT_MAX_WIDTH,
  SVEDA_CHAT_LAYOUT_MIN_HEIGHT,
  SVEDA_CHAT_LAYOUT_MIN_WIDTH,
} from '@sveda-ai/chat';
import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';

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

export function useSvedaChatLayout(isMinimized: boolean, hostEmbed: boolean, fillHost: boolean) {
  const [viewMode, setViewMode] = useState<SvedaChatViewMode>(() => readViewMode(hostEmbed));
  const [lastNonImmersiveViewMode, setLastNonImmersiveViewMode] = useState<SvedaChatViewMode>(() =>
    hostEmbed ? 'floating' : 'fixed'
  );
  const [isMobile, setIsMobile] = useState(false);
  const [chatWidth, setChatWidth] = useState(() => readChatDimensions().width);
  const [chatHeight, setChatHeight] = useState(() => readChatDimensions().height);
  const [fixedWidth, setFixedWidth] = useState(() => readNumber(FIXED_WIDTH_STORAGE_KEY, DEFAULT_FIXED_WIDTH));
  const sizeRef = useRef({ chatWidth, chatHeight, fixedWidth, viewMode });
  sizeRef.current = { chatWidth, chatHeight, fixedWidth, viewMode };

  useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
      return;
    }
    const media = window.matchMedia('(max-width: 768px)');
    const apply = () => setIsMobile(hostEmbed ? false : media.matches);
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [hostEmbed]);

  useEffect(() => {
    if (typeof document === 'undefined') {
      return;
    }
    const body = document.body;
    body.classList.remove(FIXED_MODE_BODY_CLASS, IMMERSIVE_BODY_CLASS);
    document.documentElement.style.removeProperty(CHAT_WIDTH_CSS_VAR);
    body.style.removeProperty(CHAT_WIDTH_CSS_VAR);

    if (!isMinimized && !isMobile && viewMode === 'fixed') {
      body.classList.add(FIXED_MODE_BODY_CLASS);
      body.style.setProperty(CHAT_WIDTH_CSS_VAR, `${fixedWidth}px`);
    }
    if (!isMinimized && !isMobile && viewMode === 'immersive') {
      body.classList.add(IMMERSIVE_BODY_CLASS);
    }

    return () => {
      body.classList.remove(FIXED_MODE_BODY_CLASS, IMMERSIVE_BODY_CLASS);
      body.style.removeProperty(CHAT_WIDTH_CSS_VAR);
    };
  }, [fixedWidth, isMinimized, isMobile, viewMode]);

  const saveViewMode = (mode: SvedaChatViewMode) => {
    localStorage.setItem(VIEW_MODE_STORAGE_KEY, mode);
  };

  const toggleViewMode = () => {
    if (viewMode === 'immersive') {
      return;
    }
    const next: SvedaChatViewMode = viewMode === 'floating' ? 'fixed' : 'floating';
    setViewMode(next);
    setLastNonImmersiveViewMode(next);
    saveViewMode(next);
  };

  const enterImmersiveMode = () => {
    if (isMobile || viewMode === 'immersive') {
      return;
    }
    setLastNonImmersiveViewMode(viewMode);
    setViewMode('immersive');
    saveViewMode('immersive');
  };

  const exitImmersiveMode = () => {
    if (viewMode !== 'immersive') {
      return;
    }
    const next = lastNonImmersiveViewMode === 'immersive' ? 'fixed' : lastNonImmersiveViewMode;
    setViewMode(next);
    saveViewMode(next);
  };

  const startResize = useCallback((direction: string, event: ReactPointerEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();
    const handle = event.currentTarget;
    const pointerId = event.pointerId;
    try {
      handle.setPointerCapture(pointerId);
    } catch {
      // Capture is best-effort.
    }

    const startX = event.screenX;
    const startY = event.screenY;
    const mode = sizeRef.current.viewMode;
    const startWidth = mode === 'fixed' ? sizeRef.current.fixedWidth : sizeRef.current.chatWidth;
    const startHeight = sizeRef.current.chatHeight;
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
  }, []);

  const isImmersiveDesktop = !isMobile && !isMinimized && viewMode === 'immersive';

  const chatShellClass = [
    'sveda-chat flex flex-col',
    fillHost && !isMinimized ? 'relative h-full min-h-0 w-full overflow-hidden' : '',
    !fillHost && isMinimized ? 'fixed bottom-0 right-0 z-50 min-[1872px]:bottom-4 min-[1872px]:right-4' : '',
    !fillHost && !isMinimized && isMobile
      ? 'sveda-chat-surface fixed inset-0 z-50 pb-[env(safe-area-inset-bottom)] pt-[env(safe-area-inset-top)]'
      : '',
    !fillHost && !isMinimized && !isMobile && viewMode === 'immersive'
      ? 'sveda-chat-surface fixed right-0 top-0 z-50 h-[100dvh] pb-[env(safe-area-inset-bottom)] pt-[env(safe-area-inset-top)]'
      : '',
    !fillHost && !isMinimized && !isMobile && viewMode === 'fixed'
      ? 'sveda-chat-surface fixed right-0 top-0 z-50 h-screen pt-[env(safe-area-inset-top)]'
      : '',
    !fillHost && !isMinimized && !isMobile && viewMode === 'floating' ? 'fixed bottom-4 right-4 z-50' : '',
    fillHost || isMobile || viewMode === 'fixed' || viewMode === 'immersive' ? 'items-stretch' : 'items-end gap-2',
  ]
    .filter(Boolean)
    .join(' ');

  const chatShellStyle =
    fillHost && !isMinimized
      ? { width: '100%', height: '100%' }
      : !isMobile && !isMinimized && viewMode === 'immersive'
        ? { width: '100vw', height: '100dvh' }
        : !isMobile && !isMinimized && viewMode === 'fixed'
          ? { width: 'var(--sveda-chat-width)', height: '100dvh' }
          : undefined;

  const chatCardStyle =
    isMobile || fillHost
      ? { width: '100%', height: '100%' }
      : viewMode === 'fixed'
        ? { width: 'var(--sveda-chat-width)', height: '100%' }
        : viewMode === 'floating'
          ? { width: `${chatWidth}px`, height: `${chatHeight}px` }
          : { width: '100%', height: '100%' };

  return {
    isMobile,
    viewMode,
    fixedWidth,
    setFixedWidth,
    isImmersiveDesktop,
    chatShellClass,
    chatShellStyle,
    chatCardStyle,
    chatWidth,
    chatHeight,
    startResize,
    toggleViewMode,
    enterImmersiveMode,
    exitImmersiveMode,
  };
}
