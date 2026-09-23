type ToolPart = {
  type?: string;
  toolCallId?: string;
  confirmation?: string;
};

type ToolMessage = {
  parts?: ToolPart[];
};

export function hasPendingToolConfirmation(messages: ToolMessage[] | null | undefined): boolean {
  const resolved = new Set<string>();
  for (const message of messages ?? []) {
    for (const part of message.parts ?? []) {
      if (part.type === 'tool-result' && part.toolCallId) {
        resolved.add(part.toolCallId);
      }
    }
  }

  return (messages ?? []).some(message =>
    (message.parts ?? []).some(
      part =>
        part.type === 'tool-call' &&
        part.confirmation === 'required' &&
        typeof part.toolCallId === 'string' &&
        part.toolCallId !== '' &&
        !resolved.has(part.toolCallId)
    )
  );
}
