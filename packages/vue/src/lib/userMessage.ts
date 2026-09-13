export type VedaUserMessageLike = {
  role?: string;
  content?: string;
  attachmentNames?: string[];
  metadata?: {
    attachmentNames?: string[];
  };
  parts?: Array<{
    type?: string;
    text?: string;
    name?: string;
  }>;
};

export const getUserMessageText = (message: VedaUserMessageLike): string => {
  if (typeof message.content === 'string' && message.content.trim()) {
    return message.content.trim();
  }

  if (!Array.isArray(message.parts)) {
    return '';
  }

  return message.parts
    .filter(part => part?.type === 'text')
    .map(part => String(part.text || '').trim())
    .filter(Boolean)
    .join(' ');
};

export const getUserMessageAttachmentNames = (message: VedaUserMessageLike): string[] => {
  const raw = message.attachmentNames ?? message.metadata?.attachmentNames;
  if (Array.isArray(raw)) {
    return raw.map(name => String(name).trim()).filter(Boolean);
  }

  if (!Array.isArray(message.parts)) {
    return [];
  }

  return message.parts
    .filter(part => part?.type === 'file')
    .map(part => String(part.name || '').trim())
    .filter(Boolean);
};

export const userMessageHasVisibleContent = (message: VedaUserMessageLike): boolean => {
  if (message.role !== 'user') {
    return false;
  }

  return (
    getUserMessageText(message).length > 0 || getUserMessageAttachmentNames(message).length > 0
  );
};
