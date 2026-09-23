export function shuffleArray<T>(items: T[]): T[] {
  const copy = [...items];

  for (let index = copy.length - 1; index > 0; index -= 1) {
    const swapIndex = Math.floor(Math.random() * (index + 1));
    [copy[index], copy[swapIndex]] = [copy[swapIndex], copy[index]];
  }

  return copy;
}

export function buildPendingStatusRotation(allMessages: string[]): string[] {
  const messages = allMessages.filter(message => message.trim() !== '');

  if (messages.length === 0) {
    return [];
  }

  if (messages.length === 1) {
    return [messages[0]];
  }

  const [firstMessage, ...remainingMessages] = messages;

  return [firstMessage, ...shuffleArray(remainingMessages)];
}
