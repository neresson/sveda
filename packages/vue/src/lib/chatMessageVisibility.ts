import { isAssistantToolPart, type SvedaChatMessageLike } from './assistantMessage.js';

export function messageHasVisibleAssistantContent(
  message: SvedaChatMessageLike | null | undefined
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

export function messageHasAssistantAnswerText(
  message: SvedaChatMessageLike | null | undefined
): boolean {
  if (!message || message.role !== 'assistant') {
    return false;
  }

  if (Array.isArray(message.parts) && message.parts.length > 0) {
    return message.parts.some(part => part.type === 'text' && Boolean(part.text?.trim()));
  }

  return Boolean(message.content?.trim());
}

export function assistantHasActiveTools(message: SvedaChatMessageLike | null | undefined): boolean {
  if (!message || message.role !== 'assistant' || !Array.isArray(message.parts)) {
    return false;
  }

  const completed = new Set(
    message.parts
      .filter(part => part.type === 'tool-result' && part.toolCallId)
      .map(part => part.toolCallId as string)
  );

  return message.parts.some(part => {
    if (part.type === 'tool-call') {
      return part.toolCallId ? !completed.has(part.toolCallId) : true;
    }
    if (isAssistantToolPart(part)) {
      const state = part.state ?? '';
      return (
        state !== 'result' &&
        state !== 'output-available' &&
        state !== 'output-error' &&
        state !== 'output-denied'
      );
    }
    return false;
  });
}

export function shouldShowChatPendingIndicator(
  isThinking: boolean,
  messages: SvedaChatMessageLike[]
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

  return !messageHasAssistantAnswerText(lastMessage);
}
