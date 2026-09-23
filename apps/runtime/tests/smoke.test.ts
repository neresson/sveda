import { describe, expect, it } from 'vitest';

describe('runtime admin shell', () => {
  it('exposes a document root for vitest', () => {
    expect(typeof document).toBe('object');
    expect(document.documentElement.tagName.toLowerCase()).toBe('html');
  });
});
