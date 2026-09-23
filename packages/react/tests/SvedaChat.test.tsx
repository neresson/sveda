import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { createElement } from 'react';
import { SvedaProvider, useSvedaClient } from '../src/provider';
import { SvedaChat } from '../src/components/SvedaChat';
import { getSvedaChatStore, writePersistedMinimized } from '@sveda-ai/chat';

function ClientProbe() {
  const client = useSvedaClient();
  return createElement('div', { 'data-testid': 'client-ok' }, client ? 'ok' : 'missing');
}

describe('@sveda-ai/react', () => {
  beforeEach(() => {
    cleanup();
    localStorage.clear();
    writePersistedMinimized(true);
    getSvedaChatStore().minimizeChat();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: true,
        json: async () => [],
        text: async () => '',
      }))
    );
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it('exposes a client from SvedaProvider', () => {
    render(
      createElement(
        SvedaProvider,
        { endpoints: { stream: 'http://127.0.0.1:9/sveda/stream' }, credentials: 'omit' },
        createElement(ClientProbe)
      )
    );

    expect(screen.getByTestId('client-ok').textContent).toBe('ok');
  });

  it('renders a launcher when minimized', async () => {
    render(
      createElement(
        SvedaProvider,
        {
          endpoints: { stream: 'http://127.0.0.1:9/sveda/stream' },
          credentials: 'omit',
          brand: { name: 'TestBot' },
        },
        createElement(SvedaChat)
      )
    );

    await waitFor(() => {
      expect(screen.getByTestId('sveda-launcher')).toBeTruthy();
    });
  });

  it('opens the composer when the launcher is clicked', async () => {
    const { unmount } = render(
      createElement(
        SvedaProvider,
        {
          endpoints: { stream: 'http://127.0.0.1:9/sveda/stream' },
          credentials: 'omit',
          brand: { name: 'TestBot' },
        },
        createElement(SvedaChat)
      )
    );

    const launcher = await screen.findByTestId('sveda-launcher');
    launcher.click();

    await waitFor(() => {
      expect(screen.getByTestId('sveda-composer')).toBeTruthy();
    });

    unmount();
  });

  it('switches view mode without opening a new chat and opens history', async () => {
    render(
      createElement(
        SvedaProvider,
        {
          endpoints: { stream: 'http://127.0.0.1:9/sveda/stream' },
          credentials: 'omit',
          brand: { name: 'TestBot' },
        },
        createElement(SvedaChat)
      )
    );

    const launcher = screen.queryByTestId('sveda-launcher');
    if (launcher) {
      launcher.click();
    }
    await screen.findByTestId('sveda-composer');

    const tabsBefore = screen.getAllByLabelText('Close chat tab').length;

    screen.getByLabelText('Switch to floating view').click();

    await waitFor(() => {
      expect(screen.getByLabelText('Switch to fixed view')).toBeTruthy();
    });
    expect(screen.getAllByLabelText('Close chat tab')).toHaveLength(tabsBefore);

    fireEvent.click(screen.getByRole('button', { name: 'Chat History' }));
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Search chats...')).toBeTruthy();
    });
    expect(screen.getAllByRole('button', { name: 'New Chat' }).length).toBeGreaterThan(0);
  });
});
