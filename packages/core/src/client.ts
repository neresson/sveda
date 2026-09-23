import type { SvedaRenderHint, SvedaStreamEvent, SvedaStreamRequest } from '@sveda-ai/protocol';
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
  chatId: string;

  messages: SvedaDisplayMessage[] = [];

  status: SvedaSessionStatus = 'idle';

  private readonly emitter = new SessionEmitter();

  private abortController: AbortController | null = null;

  private pendingFrontendResults: SvedaDisplayMessage[] = [];

  private pendingDecisions: Array<{ toolCallId: string; decision: 'approve' | 'deny' }> = [];

  private decisionStream = false;

  private continuationStarted = false;

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

  async resolveToolConfirmation(
    toolCallId: string,
    decision: 'approve' | 'deny',
    sendOptions: SvedaSendOptions = {}
  ): Promise<void> {
    const call = this.findToolCall(toolCallId);
    if (!call || call.confirmation !== 'required' || this.hasToolResult(toolCallId) || this.isStreaming) {
      return;
    }

    if (call.target === 'frontend') {
      const output =
        decision === 'approve'
          ? await this.runFrontendHandler(call.toolName, call.input, toolCallId)
          : deniedToolOutput();
      this.appendToolResult(toolCallId, call.toolName, output);
      await this.streamRequest('', sendOptions);
      return;
    }

    this.pendingDecisions = [{ toolCallId, decision }];
    await this.streamRequest('', sendOptions, 'decision');
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
        parts: partsForReplay(message),
      })),
      prompt,
      chatId: this.chatId,
      context,
      clientTools: this.client.toolRegistry.list(),
      ...(this.pendingDecisions.length > 0 ? { toolDecisions: this.pendingDecisions } : {}),
      model: sendOptions.model,
      provider: sendOptions.provider,
      options: sendOptions.options,
    };
  }

  private async streamRequest(
    prompt: string,
    sendOptions: SvedaSendOptions,
    mode: 'default' | 'decision' = 'default'
  ): Promise<void> {
    const requestBody = this.buildRequestBody(prompt, sendOptions);
    this.pendingDecisions = [];
    this.decisionStream = mode === 'decision';
    this.continuationStarted = false;

    this.status = 'submitted';
    this.emitter.emit('status', this.status);
    this.abortController = new AbortController();

    if (mode !== 'decision') {
      const assistantMessage: SvedaDisplayMessage = {
        id: createMessageId(),
        role: 'assistant',
        parts: [],
      };
      this.messages = [...this.messages, assistantMessage];
      this.emitter.emit('messages', this.messages);
    }

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
    } finally {
      this.decisionStream = false;
    }
  }

  private applyStreamEvent(event: SvedaStreamEvent): void {
    this.emitter.emit('event', event);

    if (
      this.decisionStream &&
      (event.type === 'message.start' ||
        event.type === 'text.delta' ||
        event.type === 'reasoning.delta' ||
        event.type === 'tool.call')
    ) {
      this.ensureContinuationAssistant(event);
    }

    if (this.decisionStream && event.type === 'tool.result') {
      this.attachToolResult(event.toolCallId, event.toolName, event.output, event.renderHint, event.renderData);
      return;
    }

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
      case 'message.start': {
        if (typeof event.chatId === 'string' && event.chatId.trim() !== '') {
          this.chatId = event.chatId;
        }
        if (typeof event.messageId === 'string' && event.messageId.trim() !== '') {
          const updated: SvedaDisplayMessage = { ...assistant, id: event.messageId };
          this.messages = [...this.messages.slice(0, -1), updated];
          this.emitter.emit('messages', this.messages);
        }
        break;
      }

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

      case 'tool.call': {
        const confirmation =
          event.confirmation === 'required' ||
          (event.target === 'frontend' &&
            this.client.toolRegistry.get(event.toolName)?.confirmation === 'required')
            ? 'required'
            : undefined;
        mutate(parts => [
          ...parts,
          {
            type: 'tool-call',
            toolCallId: event.toolCallId,
            toolName: event.toolName,
            target: event.target,
            input: event.input,
            ...(confirmation ? { confirmation } : {}),
          },
        ]);

        if (event.target === 'frontend' && confirmation !== 'required') {
          void this.executeFrontendTool(event);
        }
        break;
      }

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

  private findToolCall(toolCallId: string) {
    for (const message of this.messages) {
      for (const part of message.parts) {
        if (part.type === 'tool-call' && part.toolCallId === toolCallId) {
          return part;
        }
      }
    }
    return undefined;
  }

  private hasToolResult(toolCallId: string): boolean {
    return this.messages.some(message =>
      message.parts.some(part => part.type === 'tool-result' && part.toolCallId === toolCallId)
    );
  }

  private appendToolResult(toolCallId: string, toolName: string, output: unknown): void {
    this.attachToolResult(toolCallId, toolName, output);
  }

  private attachToolResult(
    toolCallId: string,
    toolName: string,
    output: unknown,
    renderHint?: SvedaRenderHint,
    renderData?: Record<string, unknown>
  ): void {
    if (this.hasToolResult(toolCallId)) {
      return;
    }
    const index = this.messages.findIndex(message =>
      message.parts.some(part => part.type === 'tool-call' && part.toolCallId === toolCallId)
    );
    if (index < 0) {
      return;
    }
    const message = this.messages[index];
    const updated: SvedaDisplayMessage = {
      ...message,
      parts: [
        ...message.parts,
        {
          type: 'tool-result',
          toolCallId,
          toolName,
          output,
          ...(renderHint ? { renderHint } : {}),
          ...(renderData ? { renderData } : {}),
        },
      ],
    };
    this.messages = this.messages.map((item, itemIndex) => (itemIndex === index ? updated : item));
    this.emitter.emit('messages', this.messages);
  }

  private ensureContinuationAssistant(event: SvedaStreamEvent): void {
    if (!this.continuationStarted) {
      const messageId =
        event.type === 'message.start' && typeof event.messageId === 'string' && event.messageId.trim() !== ''
          ? event.messageId
          : createMessageId();
      this.messages = [
        ...this.messages,
        { id: messageId, role: 'assistant', parts: [] },
      ];
      this.continuationStarted = true;
      this.emitter.emit('messages', this.messages);
    }
    if (event.type !== 'message.start') {
      return;
    }
    if (typeof event.chatId === 'string' && event.chatId.trim() !== '') {
      this.chatId = event.chatId;
    }
    if (typeof event.messageId === 'string' && event.messageId.trim() !== '') {
      const last = this.messages[this.messages.length - 1];
      if (last && last.role === 'assistant') {
        this.messages = [...this.messages.slice(0, -1), { ...last, id: event.messageId }];
        this.emitter.emit('messages', this.messages);
      }
    }
  }

  private async runFrontendHandler(
    toolName: string,
    input: Record<string, unknown>,
    toolCallId: string
  ): Promise<unknown> {
    try {
      return await this.client.toolRegistry.execute(toolName, input, {
        chatId: this.chatId,
        toolCallId,
      });
    } catch (error) {
      return { error: error instanceof Error ? error.message : String(error) };
    }
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

function deniedToolOutput(): Record<string, unknown> {
  return {
    success: false,
    denied: true,
    error: 'The user denied this action and it was not executed.',
  };
}

function partsForReplay(message: SvedaDisplayMessage): Array<Record<string, unknown>> {
  const parts = message.parts as Array<Record<string, unknown>>;
  if (message.role !== 'assistant') {
    return parts;
  }
  if (parts.some(part => part.type === 'reasoning')) {
    return parts;
  }
  return [{ type: 'reasoning', text: '.' }, ...parts];
}
