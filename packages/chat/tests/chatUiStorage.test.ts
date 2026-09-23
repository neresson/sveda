/** @vitest-environment happy-dom */
import { afterEach, describe, expect, it } from 'vitest';
import {
  isPersistedSessionChatOpen,
  readPersistedMinimized,
  readPersistedMinimizedPreference,
  writePersistedMinimized,
} from '../src/lib/chatUiStorage.js';

describe('chatUiStorage', () => {
  afterEach(() => {
    sessionStorage.clear();
    localStorage.clear();
  });

  it('defaults to minimized when nothing is stored', () => {
    expect(readPersistedMinimizedPreference()).toBeNull();
    expect(readPersistedMinimized()).toBe(true);
    expect(isPersistedSessionChatOpen()).toBe(false);
  });

  it('writes open state to session and local storage', () => {
    writePersistedMinimized(false);

    expect(sessionStorage.getItem('sveda.chat-minimized')).toBe('false');
    expect(localStorage.getItem('sveda.chat-minimized')).toBe('false');
    expect(readPersistedMinimized()).toBe(false);
    expect(isPersistedSessionChatOpen()).toBe(true);
  });

  it('prefers session storage over local storage for the current tab', () => {
    localStorage.setItem('sveda.chat-minimized', 'false');
    sessionStorage.setItem('sveda.chat-minimized', 'true');

    expect(readPersistedMinimized()).toBe(true);
    expect(isPersistedSessionChatOpen()).toBe(false);
  });
});
