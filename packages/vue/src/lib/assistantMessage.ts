export interface VedaResourceLink {
  label: string;
  url: string;
  icon?: string;
  kind?: 'created' | 'reference';
}

export type VedaAssistantMessagePart = {
  type?: string;
  text?: string;
  state?: string;
  toolName?: string;
  toolCallId?: string;
  target?: string;
  output?: unknown;
  result?: unknown;
  input?: unknown;
  args?: unknown;
  errorText?: string;
  renderHint?: string;
  renderData?: Record<string, unknown>;
};

export type VedaLegacyActivity = {
  phase?: string;
  steps?: Array<{
    id: string;
    label: string;
    status: string;
    detail?: string;
    kind?: string;
    activityGroupKey?: string;
    activityGroupLabel?: string;
    display?: { name?: string; link?: string };
  }>;
};

export type VedaChatMessageLike = {
  id?: string;
  role?: string;
  content?: string;
  parts?: VedaAssistantMessagePart[];
  reasoningStream?: string;
  activity?: VedaLegacyActivity;
  toolInvocations?: VedaAssistantMessagePart[];
  resources?: VedaResourceLink[];
  streaming?: boolean;
  isError?: boolean;
  attachmentNames?: string[];
  metadata?: {
    attachmentNames?: string[];
  };
};

export type AssistantMessageSegment =
  | {
      kind: 'activity';
      id: string;
      invocations: VedaAssistantMessagePart[];
      legacyActivity?: VedaLegacyActivity;
    }
  | {
      kind: 'comment';
      id: string;
      text: string;
      streaming: boolean;
    }
  | {
      kind: 'answer';
      id: string;
      text: string;
      streaming: boolean;
    }
  | {
      kind: 'reasoning';
      id: string;
      text: string;
      streaming: boolean;
    };

export function isAssistantToolPart(part: VedaAssistantMessagePart | null | undefined): boolean {
  if (!part || typeof part.type !== 'string') {
    return false;
  }
  if (part.type === 'tool-call' || part.type === 'tool-result') {
    return false;
  }
  return part.type === 'dynamic-tool' || part.type.startsWith('tool-');
}

function vedaToolPartToInvocation(
  call: VedaAssistantMessagePart | undefined,
  result: VedaAssistantMessagePart | undefined
): VedaAssistantMessagePart {
  return {
    type: 'dynamic-tool',
    toolCallId: call?.toolCallId ?? result?.toolCallId ?? '',
    toolName: call?.toolName ?? result?.toolName ?? '',
    input: call?.input,
    output: result?.output,
    renderHint: result?.renderHint,
    renderData: result?.renderData,
    state: result ? 'result' : 'call',
  };
}

export function buildAssistantMessageSegments(
  message: VedaChatMessageLike
): AssistantMessageSegment[] {
  const parts = Array.isArray(message.parts) ? message.parts : [];

  if (parts.length === 0) {
    const segments: AssistantMessageSegment[] = [];

    const legacyInvocations = Array.isArray(message.toolInvocations) ? message.toolInvocations : [];
    if (legacyInvocations.length > 0) {
      segments.push({
        kind: 'activity',
        id: 'activity-legacy-tools',
        invocations: legacyInvocations,
      });
    } else if (message.activity?.steps?.length) {
      segments.push({
        kind: 'activity',
        id: 'activity-legacy',
        invocations: [],
        legacyActivity: message.activity,
      });
    }

    if (message.reasoningStream?.trim()) {
      segments.push({
        kind: 'reasoning',
        id: 'reasoning-legacy',
        text: message.reasoningStream.trim(),
        streaming: false,
      });
    }

    if (message.content?.trim()) {
      segments.push({
        kind: 'answer',
        id: 'answer-legacy',
        text: message.content.trim(),
        streaming: false,
      });
    }

    return segments;
  }

  const resultsByCallId = new Map<string, VedaAssistantMessagePart>();
  for (const part of parts) {
    if (part.type === 'tool-result' && part.toolCallId) {
      resultsByCallId.set(part.toolCallId, part);
    }
  }
  const consumedResultIds = new Set<string>();

  const segments: AssistantMessageSegment[] = [];
  let toolBuffer: VedaAssistantMessagePart[] = [];
  const textBuffer: Array<{ part: VedaAssistantMessagePart; index: number }> = [];

  const lastTextPartIndex = parts.reduce((lastIndex, part, index) => {
    if (part.type === 'text' && part.text?.trim()) {
      return index;
    }
    return lastIndex;
  }, -1);

  const flushTools = () => {
    if (toolBuffer.length === 0) {
      return;
    }
    segments.push({
      kind: 'activity',
      id: `activity-${segments.length}`,
      invocations: [...toolBuffer],
    });
    toolBuffer = [];
  };

  const flushText = () => {
    const buffered = textBuffer.filter(
      entry => entry.part.type === 'text' && entry.part.text?.trim()
    );
    textBuffer.length = 0;
    if (buffered.length === 0) {
      return;
    }

    const text = buffered.map(entry => String(entry.part.text).trim()).join('\n\n');
    const streaming = buffered.some(entry => entry.part.state === 'streaming');
    const maxTextIndex = Math.max(...buffered.map(entry => entry.index));
    const isAnswer = lastTextPartIndex >= 0 && maxTextIndex === lastTextPartIndex;

    segments.push({
      kind: isAnswer ? 'answer' : 'comment',
      id: `text-${segments.length}`,
      text,
      streaming,
    });
  };

  for (let index = 0; index < parts.length; index += 1) {
    const part = parts[index];

    if (part.type === 'tool-call') {
      if (textBuffer.length > 0) {
        flushText();
      }
      const result = part.toolCallId ? resultsByCallId.get(part.toolCallId) : undefined;
      if (result && part.toolCallId) {
        consumedResultIds.add(part.toolCallId);
      }
      toolBuffer.push(vedaToolPartToInvocation(part, result));
      continue;
    }

    if (part.type === 'tool-result') {
      if (part.toolCallId && consumedResultIds.has(part.toolCallId)) {
        continue;
      }
      if (textBuffer.length > 0) {
        flushText();
      }
      toolBuffer.push(vedaToolPartToInvocation(undefined, part));
      continue;
    }

    if (isAssistantToolPart(part)) {
      if (textBuffer.length > 0) {
        flushText();
      }
      toolBuffer.push(part);
      continue;
    }

    if (part.type === 'reasoning' && part.text?.trim()) {
      flushTools();
      if (textBuffer.length > 0) {
        flushText();
      }
      segments.push({
        kind: 'reasoning',
        id: `reasoning-${segments.length}`,
        text: part.text.trim(),
        streaming: part.state === 'streaming',
      });
      continue;
    }

    if (part.type === 'text') {
      if (toolBuffer.length > 0) {
        flushTools();
      }
      textBuffer.push({ part, index });
      continue;
    }
  }

  flushText();
  flushTools();

  let hasAnswer = segments.some(segment => segment.kind === 'answer');
  if (!hasAnswer) {
    for (let i = segments.length - 1; i >= 0; i -= 1) {
      if (segments[i].kind === 'comment') {
        segments[i] = { ...segments[i], kind: 'answer' } as AssistantMessageSegment;
        hasAnswer = true;
        break;
      }
    }
  }

  if (!hasAnswer && message.content?.trim()) {
    segments.push({
      kind: 'answer',
      id: 'answer-content-fallback',
      text: message.content.trim(),
      streaming: false,
    });
  }

  return segments;
}

const READ_ONLY_TOOL_PREFIXES = ['get_', 'list_', 'search_', 'read_'];

export function getToolNameFromPart(part: { toolName?: string; type?: string }): string {
  if (part.toolName) {
    return part.toolName;
  }
  if (
    typeof part.type === 'string' &&
    part.type.startsWith('tool-') &&
    part.type !== 'tool-call' &&
    part.type !== 'tool-result'
  ) {
    return part.type.slice('tool-'.length);
  }
  return '';
}

export function parseToolResultOutput(output: unknown): Record<string, unknown> | null {
  if (output === undefined || output === null) {
    return null;
  }
  if (typeof output === 'object') {
    return output as Record<string, unknown>;
  }
  if (typeof output !== 'string' || output.trim() === '') {
    return null;
  }
  try {
    const parsed = JSON.parse(output) as unknown;
    return typeof parsed === 'object' && parsed !== null
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

export function getActivityGroupKey(toolName: string): string {
  if (!toolName) {
    return '';
  }
  if (READ_ONLY_TOOL_PREFIXES.some(prefix => toolName.startsWith(prefix))) {
    return `reader:${toolName}`;
  }
  return '';
}

export function getActivityGroupLabel(_toolName: string, translatedToolName: string): string {
  return translatedToolName;
}

function normalizeResourceLink(raw: unknown): VedaResourceLink | null {
  if (!raw || typeof raw !== 'object') {
    return null;
  }
  const record = raw as Record<string, unknown>;
  const label = record.label ?? record.name;
  const url = record.url ?? record.link;
  if (typeof label !== 'string' || label.trim() === '') {
    return null;
  }
  if (typeof url !== 'string' || url.trim() === '') {
    return null;
  }
  const icon = typeof record.icon === 'string' && record.icon.trim() !== '' ? record.icon.trim() : undefined;
  return { label: label.trim(), url: url.trim(), ...(icon ? { icon } : {}) };
}

function pushResourceLink(resources: VedaResourceLink[], resource: VedaResourceLink): void {
  if (!resource.label || !resource.url) {
    return;
  }
  if (resources.some(existing => existing.url === resource.url)) {
    return;
  }
  resources.push(resource);
}

export function collectResourceLinks(message: VedaChatMessageLike): VedaResourceLink[] {
  const collected: VedaResourceLink[] = [];
  const parts = [
    ...(Array.isArray(message.toolInvocations) ? message.toolInvocations : []),
    ...(Array.isArray(message.parts) ? message.parts : []),
  ];

  for (const part of parts) {
    if (!part || typeof part !== 'object') {
      continue;
    }

    if (part.type === 'tool-result' && part.renderHint === 'resource_links') {
      const links = part.renderData?.links;
      if (Array.isArray(links)) {
        for (const raw of links) {
          const link = normalizeResourceLink(raw);
          if (link) {
            pushResourceLink(collected, { ...link, kind: link.kind ?? 'reference' });
          }
        }
      }
      continue;
    }

    if (part.type === 'tool-result' || isAssistantToolPart(part)) {
      const resultData = parseToolResultOutput(part.output !== undefined ? part.output : part.result);
      if (!resultData || resultData.success === false) {
        continue;
      }

      const display = resultData.display as { name?: unknown; link?: unknown } | undefined;
      if (display?.name && display?.link) {
        pushResourceLink(collected, {
          label: String(display.name),
          url: String(display.link),
          kind: 'created',
        });
      }

      const links = resultData.links;
      if (Array.isArray(links)) {
        for (const raw of links) {
          const link = normalizeResourceLink(raw);
          if (link) {
            pushResourceLink(collected, { ...link, kind: link.kind ?? 'reference' });
          }
        }
      }
    }
  }

  return collected;
}

export function enrichMessagesWithResourceLinks<
  T extends { role?: string; resources?: VedaResourceLink[] },
>(messages: T[]): T[] {
  return messages.map(message => {
    if (message.role !== 'assistant') {
      return message;
    }
    const resources = collectResourceLinks(message);
    if (resources.length === 0) {
      return message;
    }
    return {
      ...message,
      resources,
    };
  });
}
