import { useMediaQuery, useWindowSize } from '@vueuse/core';
import { computed, inject, onBeforeUnmount, onMounted, ref, watch, type ComputedRef, type Ref } from 'vue';
import { SvedaFillHostKey, SvedaHostEmbedKey } from '../plugin';

const CHAT_STORAGE_KEY = 'sveda.chat-dimensions';
const VIEW_MODE_STORAGE_KEY = 'sveda.chat-view-mode';
const FIXED_WIDTH_STORAGE_KEY = 'sveda.chat-fixed-width';

export const SVEDA_CHAT_LAYOUT_MIN_WIDTH = 320;
export const SVEDA_CHAT_LAYOUT_MIN_HEIGHT = 400;
export const SVEDA_CHAT_LAYOUT_MAX_WIDTH = 800;
export const SVEDA_CHAT_LAYOUT_MAX_HEIGHT = 900;
export const SVEDA_CHAT_MAIN_CONTENT_MIN_WIDTH = 768;

const MIN_WIDTH = SVEDA_CHAT_LAYOUT_MIN_WIDTH;
const MIN_HEIGHT = SVEDA_CHAT_LAYOUT_MIN_HEIGHT;
const MAX_WIDTH = SVEDA_CHAT_LAYOUT_MAX_WIDTH;
const MAX_HEIGHT = SVEDA_CHAT_LAYOUT_MAX_HEIGHT;
const DEFAULT_WIDTH = 384;
const DEFAULT_HEIGHT = 600;
const DEFAULT_FIXED_WIDTH = 400;
const FIXED_RESIZE_BODY_CLASS = 'sveda-chat-fixed-resizing';
const FIXED_MODE_BODY_CLASS = 'sveda-chat-fixed-mode';
const IMMERSIVE_BODY_CLASS = 'sveda-chat-immersive-mode';
const CHAT_WIDTH_CSS_VAR = '--sveda-chat-width';
const EMBED_WIDTH_CSS_VAR = '--sveda-embed-width';
const EMBED_HEIGHT_CSS_VAR = '--sveda-embed-height';
const IMMERSIVE_MODE_TRANSITION_MS = 300;

export type SvedaChatViewMode = 'floating' | 'fixed' | 'immersive';

export const useSvedaChatLayout = (isMinimized: ComputedRef<boolean>) => {
  const hostEmbed = inject(SvedaHostEmbedKey, false);
  const fillHost = inject(SvedaFillHostKey, false);
  const isViewportMobile = useMediaQuery('(max-width: 768px)');
  const { width: windowWidth, height: windowHeight } = useWindowSize();

  const isMobile = computed(() => isViewportMobile.value);

  const chatWidth = ref(DEFAULT_WIDTH);
  const chatHeight = ref(DEFAULT_HEIGHT);
  const fixedWidth = ref(DEFAULT_FIXED_WIDTH);
  const viewMode: Ref<SvedaChatViewMode> = ref(hostEmbed ? 'floating' : 'fixed');
  const lastNonImmersiveViewMode: Ref<SvedaChatViewMode> = ref(hostEmbed ? 'floating' : 'fixed');
  const isResizing = ref(false);
  const resizeDirection = ref<string | null>(null);
  const isEnteringImmersiveFromDrag = ref(false);
  const isImmersiveModeTransitioning = ref(false);
  let immersiveFromDragTimer: ReturnType<typeof setTimeout> | null = null;
  let immersiveModeTransitionTimer: ReturnType<typeof setTimeout> | null = null;
  let immersiveModeTransitionResolve: (() => void) | null = null;

  const clearImmersiveFromDragTimer = () => {
    if (immersiveFromDragTimer != null) {
      clearTimeout(immersiveFromDragTimer);
      immersiveFromDragTimer = null;
    }
  };

  const finishImmersiveFromDrag = () => {
    clearImmersiveFromDragTimer();
    isEnteringImmersiveFromDrag.value = false;
  };

  const finishImmersiveModeTransition = () => {
    if (immersiveModeTransitionTimer != null) {
      clearTimeout(immersiveModeTransitionTimer);
      immersiveModeTransitionTimer = null;
    }
    isImmersiveModeTransitioning.value = false;
    if (immersiveModeTransitionResolve) {
      immersiveModeTransitionResolve();
      immersiveModeTransitionResolve = null;
    }
  };

  const startImmersiveModeTransition = () => {
    finishImmersiveModeTransition();
    isImmersiveModeTransitioning.value = true;
    return new Promise<void>(resolve => {
      immersiveModeTransitionResolve = resolve;
      immersiveModeTransitionTimer = setTimeout(() => {
        immersiveModeTransitionTimer = null;
        isImmersiveModeTransitioning.value = false;
        immersiveModeTransitionResolve = null;
        resolve();
      }, IMMERSIVE_MODE_TRANSITION_MS);
    });
  };

  const applyChatWidthVar = (px: number | null) => {
    document.documentElement.style.removeProperty(CHAT_WIDTH_CSS_VAR);
    if (px == null) {
      document.body.style.removeProperty(CHAT_WIDTH_CSS_VAR);
      return;
    }
    document.body.style.setProperty(CHAT_WIDTH_CSS_VAR, `${px}px`);
  };

  const maxFixedSplitWidth = computed(() =>
    Math.max(MIN_WIDTH, windowWidth.value - SVEDA_CHAT_MAIN_CONTENT_MIN_WIDTH)
  );

  const isImmersiveDesktop = computed(
    () => !isMobile.value && !isMinimized.value && viewMode.value === 'immersive'
  );

  const chatCardStyle = computed(() => {
    if (isMobile.value || fillHost) {
      return { width: '100%', height: '100%' };
    }
    if (viewMode.value === 'immersive') {
      return { width: '100%', height: '100%' };
    }
    if (viewMode.value === 'fixed') {
      return { width: 'var(--sveda-chat-width)', height: '100%' };
    }
    return { width: `${chatWidth.value}px`, height: `${chatHeight.value}px` };
  });

  const chatShellStyle = computed(() => {
    if (fillHost && !isMinimized.value) {
      return { width: '100%', height: '100%' };
    }
    if (isMobile.value || isMinimized.value) {
      return undefined;
    }
    if (viewMode.value === 'immersive') {
      return { width: '100vw', height: '100dvh' };
    }
    if (viewMode.value === 'fixed') {
      return { width: 'var(--sveda-chat-width)', height: '100dvh' };
    }
    return undefined;
  });

  const applyEmbedHostSize = () => {
    if (typeof document === 'undefined' || !fillHost) {
      return;
    }

    document.documentElement.style.setProperty(EMBED_WIDTH_CSS_VAR, `${chatWidth.value}px`);
    document.documentElement.style.setProperty(EMBED_HEIGHT_CSS_VAR, `${chatHeight.value}px`);
  };

  const clearEmbedHostSize = () => {
    if (typeof document === 'undefined' || !fillHost) {
      return;
    }

    document.documentElement.style.removeProperty(EMBED_WIDTH_CSS_VAR);
    document.documentElement.style.removeProperty(EMBED_HEIGHT_CSS_VAR);
  };

  const loadChatDimensions = () => {
    try {
      const saved = localStorage.getItem(CHAT_STORAGE_KEY);
      if (saved) {
        const { width, height } = JSON.parse(saved) as { width: number; height: number };
        chatWidth.value = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, width));
        chatHeight.value = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, height));
      }
    } catch (error) {
      console.warn('Failed to load chat dimensions:', error);
    }

    if (!hostEmbed) {
      try {
        const savedMode = localStorage.getItem(VIEW_MODE_STORAGE_KEY);
        if (
          savedMode &&
          (savedMode === 'floating' || savedMode === 'fixed' || savedMode === 'immersive')
        ) {
          viewMode.value = savedMode;
          if (savedMode === 'floating' || savedMode === 'fixed') {
            lastNonImmersiveViewMode.value = savedMode;
          }
        }
      } catch (error) {
        console.warn('Failed to load view mode:', error);
      }
    }

    try {
      const savedFixedWidth = localStorage.getItem(FIXED_WIDTH_STORAGE_KEY);
      if (savedFixedWidth) {
        const width = parseInt(savedFixedWidth, 10);
        const room =
          typeof window !== 'undefined'
            ? Math.max(MIN_WIDTH, window.innerWidth - SVEDA_CHAT_MAIN_CONTENT_MIN_WIDTH)
            : MAX_WIDTH;
        if (!isNaN(width) && width >= MIN_WIDTH) {
          fixedWidth.value = Math.min(room, width);
        }
      }
    } catch (error) {
      console.warn('Failed to load fixed width:', error);
    }
  };

  const saveChatDimensions = () => {
    try {
      localStorage.setItem(
        CHAT_STORAGE_KEY,
        JSON.stringify({
          width: chatWidth.value,
          height: chatHeight.value,
        })
      );
    } catch (error) {
      console.warn('Failed to save chat dimensions:', error);
    }
  };

  const saveViewMode = () => {
    if (hostEmbed) {
      return;
    }

    try {
      localStorage.setItem(VIEW_MODE_STORAGE_KEY, viewMode.value);
    } catch (error) {
      console.warn('Failed to save view mode:', error);
    }
  };

  const saveFixedWidth = () => {
    try {
      localStorage.setItem(FIXED_WIDTH_STORAGE_KEY, String(fixedWidth.value));
    } catch (error) {
      console.warn('Failed to save fixed width:', error);
    }
  };

  const clearFixedResizeBodyClass = () => {
    document.body.classList.remove(FIXED_RESIZE_BODY_CLASS);
  };

  const updateBodyClass = () => {
    if (hostEmbed) {
      document.body.classList.remove(IMMERSIVE_BODY_CLASS);
      document.body.classList.remove(FIXED_MODE_BODY_CLASS);
      clearFixedResizeBodyClass();
      applyChatWidthVar(null);
      return;
    }

    document.body.classList.remove(IMMERSIVE_BODY_CLASS);
    if (isMobile.value) {
      document.body.classList.remove(FIXED_MODE_BODY_CLASS);
      clearFixedResizeBodyClass();
      applyChatWidthVar(null);
      return;
    }
    if (viewMode.value === 'immersive' && !isMinimized.value) {
      document.body.classList.remove(FIXED_MODE_BODY_CLASS);
      document.body.classList.add(IMMERSIVE_BODY_CLASS);
      clearFixedResizeBodyClass();
      applyChatWidthVar(null);
      return;
    }
    if (viewMode.value === 'fixed' && !isMinimized.value) {
      document.body.classList.add(FIXED_MODE_BODY_CLASS);
      applyChatWidthVar(fixedWidth.value);
    } else {
      document.body.classList.remove(FIXED_MODE_BODY_CLASS);
      clearFixedResizeBodyClass();
      applyChatWidthVar(null);
    }
  };

  const toggleViewMode = () => {
    if (viewMode.value === 'immersive') {
      return;
    }
    viewMode.value = viewMode.value === 'floating' ? 'fixed' : 'floating';
    saveViewMode();
    updateBodyClass();
  };

  const enterImmersiveMode = () => {
    if (isMobile.value || viewMode.value === 'immersive') {
      return;
    }
    finishImmersiveFromDrag();
    void startImmersiveModeTransition();
    lastNonImmersiveViewMode.value = viewMode.value;
    viewMode.value = 'immersive';
    saveViewMode();
    updateBodyClass();
  };

  const exitImmersiveMode = async () => {
    if (viewMode.value !== 'immersive') {
      return;
    }
    finishImmersiveFromDrag();
    const transitionDone = startImmersiveModeTransition();
    viewMode.value = lastNonImmersiveViewMode.value;
    saveViewMode();
    updateBodyClass();
    await transitionDone;
  };

  const collapseImmersiveToDefaultFixedMode = async () => {
    if (viewMode.value !== 'immersive') {
      return;
    }
    finishImmersiveFromDrag();
    fixedWidth.value = Math.min(DEFAULT_FIXED_WIDTH, maxFixedSplitWidth.value);
    lastNonImmersiveViewMode.value = 'fixed';
    const transitionDone = startImmersiveModeTransition();
    viewMode.value = 'fixed';
    saveFixedWidth();
    saveViewMode();
    updateBodyClass();
    await transitionDone;
  };

  const startResize = (direction: string, event: MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    isResizing.value = true;
    resizeDirection.value = direction;

    const pointerCaptureEl =
      event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
    let pointerCaptureId: number | null = null;
    if (pointerCaptureEl?.setPointerCapture) {
      const fromEvent =
        'pointerId' in event ? Number((event as PointerEvent).pointerId) : Number.NaN;
      const candidates = Number.isFinite(fromEvent) ? [fromEvent] : [1];
      for (const id of candidates) {
        try {
          pointerCaptureEl.setPointerCapture(id);
          pointerCaptureId = id;
          break;
        } catch {
          continue;
        }
      }
    }

    const releasePointerCaptureIfAny = () => {
      if (
        pointerCaptureEl &&
        pointerCaptureId !== null &&
        typeof pointerCaptureEl.releasePointerCapture === 'function'
      ) {
        try {
          pointerCaptureEl.releasePointerCapture(pointerCaptureId);
        } catch {
          pointerCaptureId = null;
          return;
        }
      }
      pointerCaptureId = null;
    };

    const startX = event.clientX;
    const startY = event.clientY;
    const startWidth = viewMode.value === 'fixed' ? fixedWidth.value : chatWidth.value;
    const startHeight = chatHeight.value;

    if (viewMode.value === 'fixed' && direction === 'fixed-left') {
      document.body.classList.add(FIXED_RESIZE_BODY_CLASS);
      let dragFixedWidth = startWidth;
      const reservePx = SVEDA_CHAT_MAIN_CONTENT_MIN_WIDTH;
      const handleMouseMoveFixed = (e: MouseEvent) => {
        if (!isResizing.value) return;
        const deltaX = e.clientX - startX;
        const newWidth = startWidth - deltaX;
        const maxChat = window.innerWidth - reservePx;
        const endFixedResize = () => {
          releasePointerCaptureIfAny();
          isResizing.value = false;
          resizeDirection.value = null;
          clearFixedResizeBodyClass();
          saveFixedWidth();
          document.removeEventListener('mousemove', handleMouseMoveFixed);
          document.removeEventListener('mouseup', handleMouseUpFixed);
        };
        if (newWidth > maxChat && maxChat >= MIN_WIDTH) {
          dragFixedWidth = maxChat;
          fixedWidth.value = maxChat;
          lastNonImmersiveViewMode.value = 'fixed';
          applyChatWidthVar(maxChat);
          clearFixedResizeBodyClass();
          isEnteringImmersiveFromDrag.value = true;
          endFixedResize();
          const targetW = typeof window !== 'undefined' ? window.innerWidth : maxChat;
          requestAnimationFrame(() => {
            requestAnimationFrame(() => {
              applyChatWidthVar(targetW);
            });
          });
          clearImmersiveFromDragTimer();
          immersiveFromDragTimer = setTimeout(() => {
            immersiveFromDragTimer = null;
            viewMode.value = 'immersive';
            saveViewMode();
            updateBodyClass();
            finishImmersiveFromDrag();
          }, 320);
          return;
        }
        const upper = Math.max(MIN_WIDTH, maxChat);
        dragFixedWidth = Math.max(MIN_WIDTH, Math.min(upper, newWidth));
        applyChatWidthVar(dragFixedWidth);
      };

      const handleMouseUpFixed = () => {
        releasePointerCaptureIfAny();
        fixedWidth.value = dragFixedWidth;
        isResizing.value = false;
        resizeDirection.value = null;
        clearFixedResizeBodyClass();
        saveFixedWidth();
        updateBodyClass();
        document.removeEventListener('mousemove', handleMouseMoveFixed);
        document.removeEventListener('mouseup', handleMouseUpFixed);
      };

      document.addEventListener('mousemove', handleMouseMoveFixed);
      document.addEventListener('mouseup', handleMouseUpFixed);
      return;
    }

    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.value) return;

      const deltaX = e.clientX - startX;
      const deltaY = e.clientY - startY;

      if (viewMode.value === 'floating' && resizeDirection.value) {
        if (resizeDirection.value.includes('right')) {
          chatWidth.value = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth + deltaX));
        }
        if (resizeDirection.value.includes('left')) {
          chatWidth.value = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth - deltaX));
        }
        if (resizeDirection.value.includes('bottom')) {
          chatHeight.value = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, startHeight + deltaY));
        }
        if (resizeDirection.value.includes('top')) {
          chatHeight.value = Math.max(MIN_HEIGHT, Math.min(MAX_HEIGHT, startHeight - deltaY));
        }
      }
    };

    const handleMouseUp = () => {
      releasePointerCaptureIfAny();
      isResizing.value = false;
      resizeDirection.value = null;
      if (viewMode.value === 'floating') {
        saveChatDimensions();
      }
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  if (typeof window !== 'undefined') {
    loadChatDimensions();
    updateBodyClass();
    applyEmbedHostSize();
  }

  onMounted(() => {
    updateBodyClass();
    applyEmbedHostSize();
  });

  onBeforeUnmount(() => {
    clearImmersiveFromDragTimer();
    finishImmersiveModeTransition();
    isEnteringImmersiveFromDrag.value = false;
    document.body.classList.remove(IMMERSIVE_BODY_CLASS);
    document.body.classList.remove(FIXED_MODE_BODY_CLASS);
    clearFixedResizeBodyClass();
    clearEmbedHostSize();
  });

  watch([chatWidth, chatHeight], () => {
    applyEmbedHostSize();
  });

  watch([maxFixedSplitWidth, () => viewMode.value, () => isMinimized.value], () => {
    if (viewMode.value !== 'fixed' || isMinimized.value || isMobile.value) {
      return;
    }
    if (fixedWidth.value > maxFixedSplitWidth.value) {
      fixedWidth.value = maxFixedSplitWidth.value;
      saveFixedWidth();
      updateBodyClass();
    }
  });

  watch([() => viewMode.value, () => isMinimized.value, () => isMobile.value], () => {
    updateBodyClass();
  });

  watch(isMobile, mobile => {
    if (mobile && viewMode.value === 'immersive') {
      viewMode.value = lastNonImmersiveViewMode.value || 'fixed';
      saveViewMode();
      updateBodyClass();
    }
  });

  return {
    isMobile,
    windowWidth,
    windowHeight,
    chatWidth,
    chatHeight,
    fixedWidth,
    viewMode,
    lastNonImmersiveViewMode,
    isResizing,
    isEnteringImmersiveFromDrag,
    isImmersiveModeTransitioning,
    resizeDirection,
    maxFixedSplitWidth,
    isImmersiveDesktop,
    chatShellStyle,
    chatCardStyle,
    fillHost,
    startResize,
    toggleViewMode,
    enterImmersiveMode,
    exitImmersiveMode,
    collapseImmersiveToDefaultFixedMode,
    updateBodyClass,
  };
};
