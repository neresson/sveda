export const SVEDA_CHAT_MINIMIZED_STORAGE_KEY = 'sveda.chat-minimized';

const readFlag = (storage: Storage | undefined): boolean | null => {
  if (!storage) {
    return null;
  }

  try {
    const saved = storage.getItem(SVEDA_CHAT_MINIMIZED_STORAGE_KEY);
    if (saved === 'true') {
      return true;
    }
    if (saved === 'false') {
      return false;
    }
  } catch {
    return null;
  }

  return null;
};

const storageOrNull = (read: () => Storage): Storage | undefined => {
  if (typeof window === 'undefined') {
    return undefined;
  }

  try {
    return read();
  } catch {
    return undefined;
  }
};

export const readPersistedMinimizedPreference = (): boolean | null => {
  const session = readFlag(storageOrNull(() => sessionStorage));
  if (session !== null) {
    return session;
  }

  return readFlag(storageOrNull(() => localStorage));
};

export const readPersistedMinimized = (): boolean => {
  return readPersistedMinimizedPreference() ?? true;
};

export const isPersistedSessionChatOpen = (): boolean => {
  return readFlag(storageOrNull(() => sessionStorage)) === false;
};

export const writePersistedMinimized = (isMinimized: boolean): void => {
  const value = isMinimized ? 'true' : 'false';

  for (const storage of [
    storageOrNull(() => sessionStorage),
    storageOrNull(() => localStorage),
  ]) {
    if (!storage) {
      continue;
    }

    try {
      storage.setItem(SVEDA_CHAT_MINIMIZED_STORAGE_KEY, value);
    } catch {
      continue;
    }
  }
};
