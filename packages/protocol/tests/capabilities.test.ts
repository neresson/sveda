import { describe, expect, it } from 'vitest';
import {
  clientToolAllowed,
  globMatch,
  parseSvedaCapabilities,
} from '../src/index.js';

describe('globMatch', () => {
  it('matches star, prefix, suffix, and exact patterns', () => {
    expect(globMatch('anything', '*')).toBe(true);
    expect(globMatch('search_posts', 'search*')).toBe(true);
    expect(globMatch('search_posts', '*posts')).toBe(true);
    expect(globMatch('exact_tool', 'exact_tool')).toBe(true);
  });

  it('rejects empty patterns and non-matches', () => {
    expect(globMatch('search_posts', '')).toBe(false);
    expect(globMatch('search_posts', '   ')).toBe(false);
    expect(globMatch('exact_tool', 'other_tool')).toBe(false);
    expect(globMatch('prefix_only', '*missing')).toBe(false);
    expect(globMatch('only_suffix', 'missing*')).toBe(false);
  });
});

describe('parseSvedaCapabilities', () => {
  it('returns null for non-objects', () => {
    expect(parseSvedaCapabilities(null)).toBeNull();
    expect(parseSvedaCapabilities('restricted')).toBeNull();
    expect(parseSvedaCapabilities([])).toBeNull();
  });

  it('parses unrestricted capabilities', () => {
    expect(parseSvedaCapabilities({ restricted: false })).toEqual({
      restricted: false,
      web: undefined,
      code: undefined,
      mcp: undefined,
      client: undefined,
    });
  });

  it('parses restricted allow lists and max_mode aliases', () => {
    expect(
      parseSvedaCapabilities({
        restricted: true,
        web: false,
        code: true,
        mcp: {
          allow: ['search_posts', '  '],
          domains: ['docs'],
          maxMode: 'read',
        },
        client: { allow: ['ui_*', ''] },
      }),
    ).toEqual({
      restricted: true,
      web: false,
      code: true,
      mcp: {
        allow: ['search_posts'],
        domains: ['docs'],
        max_mode: 'read',
      },
      client: { allow: ['ui_*'] },
    });
  });
});

describe('clientToolAllowed', () => {
  it('allows every tool when unrestricted or capabilities are missing', () => {
    expect(clientToolAllowed('any', null)).toBe(true);
    expect(clientToolAllowed('any', { restricted: false })).toBe(true);
  });

  it('blocks tools outside the restricted allow list', () => {
    const caps = parseSvedaCapabilities({
      restricted: true,
      client: { allow: ['allowed_tool', 'ui_*'] },
    });
    expect(clientToolAllowed('allowed_tool', caps)).toBe(true);
    expect(clientToolAllowed('ui_confirm', caps)).toBe(true);
    expect(clientToolAllowed('blocked_tool', caps)).toBe(false);
  });

  it('blocks all client tools when restricted allow is empty', () => {
    const caps = parseSvedaCapabilities({ restricted: true, client: { allow: [] } });
    expect(clientToolAllowed('any', caps)).toBe(false);
  });
});
