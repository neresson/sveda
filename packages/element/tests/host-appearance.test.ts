import { afterEach, describe, expect, it, vi } from 'vitest';
import { mergeSvedaAppearance } from '@sveda-ai/vue';
import { requestHostSession, resolveSvedaSession } from '../src/session';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('host session appearance', () => {
  it('forwards appearance from POST /sveda/session', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(
          JSON.stringify({
            origin: 'http://127.0.0.1:8787',
            token: 'sveda_embed_host',
            appearance: { preset: 'lms' },
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        ),
      ),
    );

    await expect(requestHostSession('/sveda/session')).resolves.toEqual({
      origin: 'http://127.0.0.1:8787',
      token: 'sveda_embed_host',
      appearance: { preset: 'lms' },
    });
  });

  it('treats a missing host appearance as no override', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(
          JSON.stringify({
            origin: 'http://127.0.0.1:8787',
            token: 'sveda_embed_host',
            appearance: null,
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        ),
      ),
    );

    const session = await requestHostSession('/sveda/session');
    expect(session?.appearance).toBeNull();
    expect(mergeSvedaAppearance({ preset: 'sand' }, session?.appearance)).toEqual({ preset: 'sand' });
  });

  it('lets session appearance overlay admin embed config', () => {
    expect(mergeSvedaAppearance({ preset: 'lms', radius: '8px' }, { preset: 'ocean' })).toEqual({
      preset: 'ocean',
      radius: '8px',
    });
  });
});

describe('resolveSvedaSession', () => {
  it('does not invent appearance when origin and token are attributes', async () => {
    const element = document.createElement('sveda-chat');
    element.setAttribute('origin', 'http://127.0.0.1:8787');
    element.setAttribute('token', 'sveda_embed_attr');

    await expect(resolveSvedaSession(element)).resolves.toEqual({
      origin: 'http://127.0.0.1:8787',
      token: 'sveda_embed_attr',
    });
  });
});
