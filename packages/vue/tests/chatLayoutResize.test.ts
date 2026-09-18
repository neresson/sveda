import { describe, expect, it } from 'vitest';
import {
  applyFixedLeftResize,
  applyFloatingResize,
  pointerScreenDelta,
  resolveEmbedHostSize,
  SVEDA_CHAT_LAYOUT_MAX_WIDTH,
  SVEDA_CHAT_LAYOUT_MIN_HEIGHT,
  SVEDA_CHAT_LAYOUT_MIN_WIDTH,
  SVEDA_HISTORY_SIDEBAR_WIDTH,
} from '../src/lib/chatResize';

describe('chatResize', () => {
  it('grows the floating window from the left/top using screen deltas', () => {
    expect(applyFloatingResize('left', 384, 600, -40, 0)).toEqual({ width: 424, height: 600 });
    expect(applyFloatingResize('top', 384, 600, 0, -50)).toEqual({ width: 384, height: 650 });
    expect(applyFloatingResize('top-left', 384, 600, -40, -50)).toEqual({
      width: 424,
      height: 650,
    });
  });

  it('clamps floating resize to layout bounds', () => {
    expect(applyFloatingResize('left', 384, 600, 400, 0).width).toBe(SVEDA_CHAT_LAYOUT_MIN_WIDTH);
    expect(applyFloatingResize('left', 384, 600, -2000, 0).width).toBe(SVEDA_CHAT_LAYOUT_MAX_WIDTH);
    expect(applyFloatingResize('top', 384, 600, 0, 400).height).toBe(SVEDA_CHAT_LAYOUT_MIN_HEIGHT);
  });

  it('keeps the size stable when the iframe origin shifts but the screen pointer does not', () => {
    const startWidth = 384;
    const startScreenX = 1000;
    const pointerScreenX = 960;

    const afterMove = applyFloatingResize(
      'left',
      startWidth,
      600,
      pointerScreenDelta({ screenX: pointerScreenX, screenY: 500 }, startScreenX, 500).deltaX,
      0
    );
    expect(afterMove.width).toBe(424);

    const afterIframeOriginShift = applyFloatingResize(
      'left',
      startWidth,
      600,
      pointerScreenDelta({ screenX: pointerScreenX, screenY: 500 }, startScreenX, 500).deltaX,
      0
    );
    expect(afterIframeOriginShift.width).toBe(424);
  });

  it('snaps back if clientX is reused after a bottom-right iframe grows', () => {
    expect(applyFloatingResize('left', 384, 600, -40, 0).width).toBe(424);
    expect(applyFloatingResize('left', 384, 600, 0, 0).width).toBe(384);
  });

  it('enters immersive when a fixed-left drag exceeds the remaining page width', () => {
    expect(applyFixedLeftResize(400, -500, 800)).toEqual({ width: 800, enterImmersive: true });
    expect(applyFixedLeftResize(400, 20, 800)).toEqual({ width: 380, enterImmersive: false });
  });

  it('adds history width to the embed host and fills the viewport in immersive mode', () => {
    expect(
      resolveEmbedHostSize({
        chatWidth: 384,
        chatHeight: 600,
        fixedWidth: 400,
        historyOpen: false,
        viewMode: 'floating',
        viewportWidth: 1440,
        viewportHeight: 900,
      })
    ).toEqual({ width: 384, height: 600, immersive: false, fixed: false });

    expect(
      resolveEmbedHostSize({
        chatWidth: 384,
        chatHeight: 600,
        fixedWidth: 400,
        historyOpen: true,
        viewMode: 'floating',
        viewportWidth: 1440,
        viewportHeight: 900,
      })
    ).toEqual({ width: 384 + SVEDA_HISTORY_SIDEBAR_WIDTH, height: 600, immersive: false, fixed: false });

    expect(
      resolveEmbedHostSize({
        chatWidth: 384,
        chatHeight: 600,
        fixedWidth: 400,
        historyOpen: true,
        viewMode: 'fixed',
        viewportWidth: 1440,
        viewportHeight: 900,
      })
    ).toEqual({
      width: 400 + SVEDA_HISTORY_SIDEBAR_WIDTH,
      height: 900,
      immersive: false,
      fixed: true,
    });

    expect(
      resolveEmbedHostSize({
        chatWidth: 384,
        chatHeight: 600,
        fixedWidth: 400,
        historyOpen: true,
        viewMode: 'immersive',
        viewportWidth: 1440,
        viewportHeight: 900,
      })
    ).toEqual({ width: 1440, height: 900, immersive: true, fixed: false });
  });
});
