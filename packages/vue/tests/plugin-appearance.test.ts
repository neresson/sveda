import { afterEach, describe, expect, it, vi } from 'vitest';
import { SVEDA_APPEARANCE_STYLE_ID } from '../src/appearance';
import { createSveda } from '../src/plugin';

afterEach(() => {
  document.getElementById(SVEDA_APPEARANCE_STYLE_ID)?.remove();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('createSveda appearance', () => {
  it('applies host JS appearance immediately and does not fetch admin config', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    createSveda({
      endpoints: { stream: 'http://127.0.0.1:8787/sveda/stream' },
      appearance: { preset: 'lms' },
    });
    await Promise.resolve();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(document.getElementById(SVEDA_APPEARANCE_STYLE_ID)?.textContent).toContain(
      '--sveda-brand:275 96% 52%',
    );
  });

  it('treats an empty appearance object as a host payload and skips admin fetch', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    createSveda({
      endpoints: { stream: 'http://127.0.0.1:8787/sveda/stream' },
      appearance: {},
    });
    await Promise.resolve();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(document.getElementById(SVEDA_APPEARANCE_STYLE_ID)?.textContent).toContain(
      '--sveda-brand:0 0% 4%',
    );
  });

  it('does not fetch embed config for a relative stream URL', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    createSveda({ endpoints: { stream: '/sveda/stream' } });
    await Promise.resolve();

    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('loads admin presets when the host omits appearance on an absolute stream URL', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ appearance: { preset: 'ocean' } }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    createSveda({ endpoints: { stream: 'http://127.0.0.1:8787/sveda/stream' } });

    await vi.waitFor(() => {
      expect(document.getElementById(SVEDA_APPEARANCE_STYLE_ID)?.textContent).toContain(
        '--sveda-brand:221 83% 53%',
      );
    });
  });
});
