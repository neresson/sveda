export const VEDA_CHAT_MINIMIZED_STORAGE_KEY = 'veda.chat-minimized';

export const readPersistedMinimized = (): boolean => {
  if (typeof window === 'undefined') {
    return true;
  }

  try {
    const saved = localStorage.getItem(VEDA_CHAT_MINIMIZED_STORAGE_KEY);
    if (saved === 'true') {
      return true;
    }
    if (saved === 'false') {
      return false;
    }
  } catch {
    return true;
  }

  return true;
};

export const writePersistedMinimized = (isMinimized: boolean): void => {
  if (typeof window === 'undefined') {
    return;
  }

  try {
    localStorage.setItem(VEDA_CHAT_MINIMIZED_STORAGE_KEY, isMinimized ? 'true' : 'false');
  } catch {
    return;
  }
};
