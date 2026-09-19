import { describe, expect, it } from 'vitest';
import { isSvedaAppearanceProvided, mergeSvedaAppearance } from '../src/appearance';

describe('mergeSvedaAppearance', () => {
  it('treats empty objects as not provided', () => {
    expect(isSvedaAppearanceProvided({})).toBe(false);
    expect(isSvedaAppearanceProvided(null)).toBe(false);
    expect(mergeSvedaAppearance({}, null)).toBeNull();
  });

  it('keeps admin presets when the host passes nothing', () => {
    expect(mergeSvedaAppearance({ preset: 'lms' }, null)).toEqual({ preset: 'lms' });
    expect(mergeSvedaAppearance({ preset: 'lms' }, {})).toEqual({ preset: 'lms' });
  });

  it('lets host JS overlay admin fields', () => {
    expect(
      mergeSvedaAppearance(
        { preset: 'default', radius: '0px', launcher: { label: 'ADMIN' } },
        { preset: 'ocean', launcher: { label: 'HELP' } },
      ),
    ).toEqual({
      preset: 'ocean',
      radius: '0px',
      launcher: { label: 'HELP' },
    });
  });

  it('drops admin tokens when the host sets a named preset', () => {
    expect(
      mergeSvedaAppearance(
        { preset: 'sand', tokens: { brand: '30 20% 20%' } },
        { preset: 'lms' },
      ),
    ).toEqual({ preset: 'lms' });
  });
});
