import { createApp, h, ref } from 'vue';
import { expect, it, vi } from 'vitest';
import ChatInput from '../src/components/chat/ChatInput.vue';
import { useSvedaStreaming } from '../src/composables/useSvedaStreaming';
import { createSveda } from '../src/plugin';

it('allows another send after stopping before response headers arrive', async () => {
  const firstRequest = vi.fn((_url: string, init: RequestInit) =>
    new Promise<Response>((_resolve, reject) => {
      init.signal?.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')));
    })
  );
  const fetch = vi.fn()
    .mockImplementationOnce(firstRequest)
    .mockResolvedValueOnce(new Response(''));
  vi.stubGlobal('fetch', fetch);

  const root = document.createElement('div');
  const app = createApp({
    setup() {
      const stream = useSvedaStreaming({
        currentChat: ref({ id: 'active-chat' }),
        setChatMessages: vi.fn(),
        setChatTitle: vi.fn(),
        refreshChatTitleFromServer: vi.fn(),
        incrementChatTokens: vi.fn(),
        setChatContextWindowTokens: vi.fn(),
      }, vi.fn());
      return () => h(ChatInput, {
        hasCurrentChat: true,
        modelValue: 'Hello',
        isStreaming: stream.isStreaming.value,
        onSend: () => stream.sendMessage('Hello'),
        onStop: () => stream.stopStreaming(),
      });
    },
  });
  app.use(createSveda({ endpoints: { stream: '/stream' }, locale: 'ru' }));
  try {
    app.mount(root);
    const button = (label: string) => root.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);
    button('Отправить сообщение')!.click();
    await vi.waitFor(() => expect(button('Остановить генерацию')).not.toBeNull());
    button('Остановить генерацию')!.click();
    await vi.waitFor(() => expect(button('Отправить сообщение')).not.toBeNull());
    expect(firstRequest.mock.calls[0][1].signal?.aborted).toBe(true);
    button('Отправить сообщение')!.click();
    await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(2));
  } finally {
    app.unmount();
    vi.unstubAllGlobals();
  }
});
