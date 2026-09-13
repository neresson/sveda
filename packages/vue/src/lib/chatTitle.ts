export const isChatTitlePlaceholder = (title: string, newChatLabel: string): boolean => {
  const normalized = (title || '').trim();
  if (!normalized || normalized === '__NEW_CHAT__' || normalized === 'New Chat') {
    return true;
  }

  if (normalized === newChatLabel) {
    return true;
  }

  if (normalized.startsWith(`${newChatLabel} `)) {
    const suffix = normalized.slice(newChatLabel.length).trim();
    return /^\d+$/.test(suffix);
  }

  return false;
};

export const deriveProvisionalChatTitle = (userRequest: string): string => {
  const text = userRequest.replace(/\s+/g, ' ').trim();
  if (!text) {
    return '';
  }

  if (text.length > 80) {
    return text.slice(0, 80).trimEnd();
  }

  return text;
};
