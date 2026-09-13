import { isAssistantToolPart, type VedaChatMessageLike } from './assistantMessage.js';

export function messageHasVisibleAssistantContent(
  message: VedaChatMessageLike | null | undefined
): boolean {
  if (!message || message.role !== 'assistant') {
    return false;
  }

  if (Array.isArray(message.parts) && message.parts.length > 0) {
    return message.parts.some(part => {
      if (part.type === 'text' && part.text?.trim()) {
        return true;
      }
      if (part.type === 'reasoning' && part.text?.trim()) {
        return true;
      }
      if (part.type === 'tool-call' || part.type === 'tool-result') {
        return true;
      }
      if (isAssistantToolPart(part)) {
        return true;
      }

      return false;
    });
  }

  return Boolean(message.content?.trim() || message.reasoningStream?.trim());
}

export function shouldShowChatPendingIndicator(
  isThinking: boolean,
  messages: VedaChatMessageLike[]
): boolean {
  if (!isThinking) {
    return false;
  }

  const lastMessage = messages[messages.length - 1];
  if (!lastMessage) {
    return true;
  }

  if (lastMessage.role === 'user') {
    return true;
  }

  return !messageHasVisibleAssistantContent(lastMessage);
}
