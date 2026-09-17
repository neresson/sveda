import type { SvedaStreamEvent, SvedaStreamRequest } from '@sveda-ai/protocol';
import { SvedaContextRegistry } from './context.js';
import { iterateSvedaStream, type SvedaStreamProtocolMode } from './streaming.js';
import { SvedaToolRegistry } from './tools.js';
import {
  createMessageId,
  messageText,
  type SvedaChatHistoryDetail,
  type SvedaChatHistorySummary,
  type SvedaDisplayMessage,
  type SvedaMessagePart,
  type SvedaSessionStatus,
} from './types.js';

export interface SvedaClientEndpoints {
  stream: string;
  message?: string;
  histories?: string;
  history?: (chatId: string) => string;
  documentsExtract?: string;
}

export interface SvedaClientOptions {
  endpoints: SvedaClientEndpoints;
  protocolMode?: SvedaStreamProtocolMode;
  headers?: Record<string, string> | (() => Record<string, string>);
  credentials?: RequestCredentials;
  tools?: SvedaToolRegistry;
  context?: SvedaContextRegistry;
  autoSubmitFrontendToolResults?: boolean;
  fetchFn?: typeof fetch;
}

export interface SvedaSendOptions {
  context?: Record<string, unknown>;
  model?: string;
  provider?: string;
  options?: Record<string, unknown>;
  files?: Array<{ name: string; mediaType?: string; url?: string }>;
}

type SessionListener = (payload: unknown) => void;

class SessionEmitter {
  private listeners = new Map<string, Set<SessionListener>>();

  on(event: string, listener: SessionListener): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(listener);

    return () => this.off(event, listener);
  }

  off(event: string, listener: SessionListener): void {
    this.listeners.get(event)?.delete(listener);
  }

  emit(event: string, payload?: unknown): void {
    for (const listener of this.listeners.get(event) ?? []) {
      listener(payload);
    }
  }
}

export class SvedaChatSession {
  readonly chatId: string;

  messages: SvedaDisplayMessage[] = [];

  status: SvedaSessionStatus = 'idle';

  private readonly emitter = new SessionEmitter();

  private abortController: AbortController | null = null;

  private pendingFrontendResults: SvedaDisplayMessage[] = [];

  constructor(
    chatId: string,
    private readonly client: SvedaClient
  ) {
    this.chatId = chatId;
  }

  on(event: string, listener: SessionListener): () => void {
    return this.emitter.on(event, listener);
  }

  get isStreaming(): boolean {
    return this.status === 'streaming' || this.status === 'submitted';
  }

  setMessages(messages: SvedaDisplayMessage[]): void {
    this.messages = messages;
    this.emitter.emit('messages', this.messages);
  }

  stop(): void {
    this.abortController?.abort();
  }

  async send(text: string, sendOptions: SvedaSendOptions = {}): Promise<void> {
    const userMessage: SvedaDisplayMessage = {
      id: createMessageId(),
      role: 'user',
      parts: [
        { type: 'text', text },
        ...(sendOptions.files ?? []).map(
          (file): SvedaMessagePart => ({
            type: 'file',
            name: file.name,
            mediaType: file.mediaType,
            url: file.url,
          })
        ),
      ],
    };

    this.messages = [...this.messages, userMessage];
    this.emitter.emit('messages', this.messages);

    await this.streamRequest(text, sendOptions);
  }

  async continueAfterMaxSteps(sendOptions: SvedaSendOptions = {}): Promise<void> {
    await this.streamRequest('Continue', sendOptions);
  }

  private buildRequestBody(prompt: string, sendOptions: SvedaSendOptions): SvedaStreamRequest {
    const context: Record<string, unknown> = {
      ...this.client.contextRegistry.snapshot(),
      ...(sendOptions.context ?? {}),
    };

    return {
      messages: this.messages.map(message => ({
        id: message.id,
        role: message.role,
        content: messageText(message),
        parts: message.parts as Array<Record<string, unknown>>,
      })),
      prompt,
      chatId: this.chatId,
      context,
      clientTools: this.client.toolRegistry.list(),
      model: sendOptions.model,
      provider: sendOptions.provider,
      options: sendOptions.options,
    };
  }

  private async streamRequest(prompt: string, sendOptions: SvedaSendOptions): Promise<void> {
    const requestBody = this.buildRequestBody(prompt, sendOptions);

    this.status = 'submitted';
    this.emitter.emit('status', this.status);
    this.abortController = new AbortController();

    const assistantMessage: SvedaDisplayMessage = {
      id: createMessageId(),
      role: 'assistant',
      parts: [],
    };
    this.messages = [...this.messages, assistantMessage];
    this.emitter.emit('messages', this.messages);

    try {
      const response = await this.client.fetchImpl(this.client.endpoints.stream, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...this.client.streamAcceptHeaders(),
          ...this.client.resolveHeaders(),
        },
        credentials: this.client.credentials,
        body: JSON.stringify(requestBody),
        signal: this.abortController.signal,
      });

      if (!response.ok) {
        throw new Error(`[sveda] Stream request failed: ${response.status}`);
      }

      this.status = 'streaming';
      this.emitter.emit('status', this.status);

      for await (const event of iterateSvedaStream(response, this.client.protocolMode)) {
        this.applyStreamEvent(event);
      }

      this.status = 'idle';
      this.emitter.emit('status', this.status);

      if (
        this.client.autoSubmitFrontendToolResults &&
        this.pendingFrontendResults.length > 0
      ) {
        const results = this.pendingFrontendResults;
        this.pendingFrontendResults = [];
        this.messages = [...this.messages, ...results];
        this.emitter.emit('messages', this.messages);
        await this.streamRequest('tool_results', sendOptions);
      }
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        this.status = 'idle';
        this.emitter.emit('status', this.status);
        return;
      }

      this.status = 'error';
      this.emitter.emit('status', this.status);
      this.emitter.emit('error', error);
      throw error;
    }
  }

  private applyStreamEvent(event: SvedaStreamEvent): void {
    this.emitter.emit('event', event);

    const assistant = this.messages[this.messages.length - 1];
    if (!assistant || assistant.role !== 'assistant') {
      return;
    }

    const mutate = (mutator: (parts: SvedaMessagePart[]) => SvedaMessagePart[]) => {
      const updated: SvedaDisplayMessage = {
        ...assistant,
        parts: mutator(assistant.parts),
      };
      this.messages = [...this.messages.slice(0, -1), updated];
      this.emitter.emit('messages', this.messages);
    };

    switch (event.type) {
      case 'text.delta':
        mutate(parts => {
          const last = parts[parts.length - 1];
          if (last && last.type === 'text') {
            return [...parts.slice(0, -1), { ...last, text: last.text + event.delta }];
          }
          return [...parts, { type: 'text', text: event.delta }];
        });
        break;

      case 'reasoning.delta':
        mutate(parts => {
          const last = parts[parts.length - 1];
          if (last && last.type === 'reasoning') {
            return [...parts.slice(0, -1), { ...last, text: last.text + event.delta }];
          }
          return [...parts, { type: 'reasoning', text: event.delta }];
        });
        break;

      case 'tool.call':
        mutate(parts => [
          ...parts,
          {
            type: 'tool-call',
            toolCallId: event.toolCallId,
            toolName: event.toolName,
            target: event.target,
            input: event.input,
          },
        ]);

        if (event.target === 'frontend') {
          void this.executeFrontendTool(event);
        }
        break;

      case 'tool.result':
        mutate(parts => [
          ...parts,
          {
            type: 'tool-result',
            toolCallId: event.toolCallId,
            toolName: event.toolName,
            output: event.output,
            renderHint: event.renderHint,
            renderData: event.renderData,
          },
        ]);
        break;

      case 'tool.progress':
        this.emitter.emit('toolProgress', event);
        break;

      case 'context.usage':
        this.emitter.emit('contextUsage', event);
        break;

      case 'chat.title':
        this.emitter.emit('title', event.title);
        break;

      case 'max_steps':
        this.emitter.emit('maxSteps', event);
        break;

      case 'message.end':
        this.emitter.emit('finish', event);
        break;

      case 'error':
        this.status = 'error';
        this.emitter.emit('status', this.status);
        this.emitter.emit('error', event);
        this.abortController?.abort();
        break;
    }
  }

  private async executeFrontendTool(
    event: Extract<SvedaStreamEvent, { type: 'tool.call' }>
  ): Promise<void> {
    let output: unknown;
    try {
      output = await this.client.toolRegistry.execute(event.toolName, event.input, {
        chatId: this.chatId,
        toolCallId: event.toolCallId,
      });
    } catch (error) {
      output = { error: error instanceof Error ? error.message : String(error) };
    }

    this.pendingFrontendResults.push({
      id: createMessageId(),
      role: 'tool',
      parts: [
        {
          type: 'tool-result',
          toolCallId: event.toolCallId,
          toolName: event.toolName,
          output,
        },
      ],
    });
  }
}

export class SvedaClient {
  readonly endpoints: SvedaClientEndpoints;

  readonly protocolMode: SvedaStreamProtocolMode;

  readonly toolRegistry: SvedaToolRegistry;

  readonly contextRegistry: SvedaContextRegistry;

  readonly autoSubmitFrontendToolResults: boolean;

  readonly fetchImpl: typeof fetch;

  readonly credentials?: RequestCredentials;

  private readonly headers?: Record<string, string> | (() => Record<string, string>);

  private sessions = new Map<string, SvedaChatSession>();

  constructor(options: SvedaClientOptions) {
    this.endpoints = options.endpoints;
    this.protocolMode = options.protocolMode ?? 'sveda';
    this.headers = options.headers;
    this.credentials = options.credentials;
    this.toolRegistry = options.tools ?? new SvedaToolRegistry();
    this.contextRegistry = options.context ?? new SvedaContextRegistry();
    this.autoSubmitFrontendToolResults = options.autoSubmitFrontendToolResults ?? true;
    this.fetchImpl = options.fetchFn ?? ((input, init) => globalThis.fetch(input, init));
  }

  resolveHeaders(): Record<string, string> {
    if (typeof this.headers === 'function') {
      return this.headers();
    }

    return this.headers ?? {};
  }

  streamAcceptHeaders(): Record<string, string> {
    if (this.protocolMode === 'sveda') {
      return { Accept: 'application/vnd.sveda.stream+json' };
    }

    return { Accept: 'text/event-stream', 'X-Sveda-Protocol': 'vercel' };
  }

  session(chatId: string): SvedaChatSession {
    if (!this.sessions.has(chatId)) {
      this.sessions.set(chatId, new SvedaChatSession(chatId, this));
    }

    return this.sessions.get(chatId)!;
  }

  createSession(chatId: string = createMessageId()): SvedaChatSession {
    return this.session(chatId);
  }

  destroySession(chatId: string): void {
    this.sessions.get(chatId)?.stop();
    this.sessions.delete(chatId);
  }

  private historiesUrl(): string {
    if (!this.endpoints.histories) {
      throw new Error('[sveda] histories endpoint is not configured');
    }

    return this.endpoints.histories;
  }

  private historyUrl(chatId: string): string {
    if (this.endpoints.history) {
      return this.endpoints.history(chatId);
    }

    return `${this.historiesUrl().replace(/\/$/, '')}/${encodeURIComponent(chatId)}`;
  }

  async listHistories(): Promise<SvedaChatHistorySummary[]> {
    const response = await this.fetchImpl(this.historiesUrl(), {
      headers: { Accept: 'application/json', ...this.resolveHeaders() },
      credentials: this.credentials,
    });

    if (!response.ok) {
      throw new Error(`[sveda] Failed to load chat histories: ${response.status}`);
    }

    const data = (await response.json()) as { histories?: SvedaChatHistorySummary[] };
    return (data.histories ?? []).map(history => this.normalizeHistory(history));
  }

  async getHistory(chatId: string): Promise<SvedaChatHistoryDetail> {
    const response = await this.fetchImpl(this.historyUrl(chatId), {
      headers: { Accept: 'application/json', ...this.resolveHeaders() },
      credentials: this.credentials,
    });

    if (!response.ok) {
      throw new Error(`[sveda] Failed to load chat history ${chatId}: ${response.status}`);
    }

    const data = (await response.json()) as { history: SvedaChatHistoryDetail };
    return this.normalizeHistory(data.history);
  }

  async renameHistory(chatId: string, title: string): Promise<void> {
    const response = await this.fetchImpl(this.historyUrl(chatId), {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
        ...this.resolveHeaders(),
      },
      credentials: this.credentials,
      body: JSON.stringify({ title }),
    });

    if (!response.ok) {
      throw new Error(`[sveda] Failed to rename chat ${chatId}: ${response.status}`);
    }
  }

  async deleteHistory(chatId: string): Promise<void> {
    const response = await this.fetchImpl(this.historyUrl(chatId), {
      method: 'DELETE',
      headers: { Accept: 'application/json', ...this.resolveHeaders() },
      credentials: this.credentials,
    });

    if (!response.ok) {
      throw new Error(`[sveda] Failed to delete chat ${chatId}: ${response.status}`);
    }
  }

  private normalizeHistory<T extends SvedaChatHistorySummary>(history: T): T {
    const fromChatId = typeof history.chatId === 'string' ? history.chatId.trim() : '';
    if (fromChatId !== '') {
      return { ...history, chatId: fromChatId };
    }

    const fromId = typeof history.id === 'string' ? history.id.trim() : '';

    return { ...history, chatId: fromId };
  }
}
