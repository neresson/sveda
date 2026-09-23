export const SVEDA_CHAT_LAYOUT_MIN_WIDTH = 320;
export const SVEDA_CHAT_LAYOUT_MIN_HEIGHT = 400;
export const SVEDA_CHAT_LAYOUT_MAX_WIDTH = 800;
export const SVEDA_CHAT_LAYOUT_MAX_HEIGHT = 900;
export const SVEDA_HISTORY_SIDEBAR_WIDTH = 320;

export interface EmbedHostSize {
  width: number;
  height: number;
  immersive: boolean;
  fixed: boolean;
}

export const resolveEmbedHostSize = ({
  chatWidth,
  chatHeight,
  fixedWidth,
  historyOpen,
  viewMode,
  viewportWidth,
  viewportHeight,
}: {
  chatWidth: number;
  chatHeight: number;
  fixedWidth: number;
  historyOpen: boolean;
  viewMode: 'floating' | 'fixed' | 'immersive';
  viewportWidth: number;
  viewportHeight: number;
}): EmbedHostSize => {
  if (viewMode === 'immersive') {
    return {
      width: viewportWidth,
      height: viewportHeight,
      immersive: true,
      fixed: false,
    };
  }

  if (viewMode === 'fixed') {
    return {
      width: fixedWidth + (historyOpen ? SVEDA_HISTORY_SIDEBAR_WIDTH : 0),
      height: viewportHeight,
      immersive: false,
      fixed: true,
    };
  }

  return {
    width: chatWidth + (historyOpen ? SVEDA_HISTORY_SIDEBAR_WIDTH : 0),
    height: chatHeight,
    immersive: false,
    fixed: false,
  };
};

const clamp = (value: number, min: number, max: number): number =>
  Math.max(min, Math.min(max, value));

export interface ChatBoxSize {
  width: number;
  height: number;
}

/**
 * Pointer deltas for window resize must use screen coordinates.
 *
 * Inside an iframe, `clientX`/`clientY` are relative to the iframe viewport.
 * Hosts pin the frame to the bottom-right, so growing the frame moves its origin
 * and the next `clientX` snaps back — the window jitters. `screenX`/`screenY`
 * stay stable across that origin shift (and still work for the in-page JS chat).
 */
export const pointerScreenDelta = (
  event: Pick<MouseEvent, 'screenX' | 'screenY'>,
  startX: number,
  startY: number
): { deltaX: number; deltaY: number } => ({
  deltaX: event.screenX - startX,
  deltaY: event.screenY - startY,
});

export const applyFloatingResize = (
  direction: string,
  startWidth: number,
  startHeight: number,
  deltaX: number,
  deltaY: number
): ChatBoxSize => {
  let width = startWidth;
  let height = startHeight;

  if (direction.includes('right')) {
    width = clamp(startWidth + deltaX, SVEDA_CHAT_LAYOUT_MIN_WIDTH, SVEDA_CHAT_LAYOUT_MAX_WIDTH);
  }
  if (direction.includes('left')) {
    width = clamp(startWidth - deltaX, SVEDA_CHAT_LAYOUT_MIN_WIDTH, SVEDA_CHAT_LAYOUT_MAX_WIDTH);
  }
  if (direction.includes('bottom')) {
    height = clamp(startHeight + deltaY, SVEDA_CHAT_LAYOUT_MIN_HEIGHT, SVEDA_CHAT_LAYOUT_MAX_HEIGHT);
  }
  if (direction.includes('top')) {
    height = clamp(startHeight - deltaY, SVEDA_CHAT_LAYOUT_MIN_HEIGHT, SVEDA_CHAT_LAYOUT_MAX_HEIGHT);
  }

  return { width, height };
};

export const applyFixedLeftResize = (
  startWidth: number,
  deltaX: number,
  maxChat: number
): { width: number; enterImmersive: boolean } => {
  const next = startWidth - deltaX;
  if (next > maxChat && maxChat >= SVEDA_CHAT_LAYOUT_MIN_WIDTH) {
    return { width: maxChat, enterImmersive: true };
  }

  const upper = Math.max(SVEDA_CHAT_LAYOUT_MIN_WIDTH, maxChat);
  return {
    width: clamp(next, SVEDA_CHAT_LAYOUT_MIN_WIDTH, upper),
    enterImmersive: false,
  };
};
