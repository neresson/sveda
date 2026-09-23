import { afterEach, describe, expect, it } from 'vitest';
import {
  applySvedaAppearance,
  buildAppearanceCss,
  isSvedaAppearanceProvided,
  mergeSvedaAppearance,
  resolveSvedaAppearance,
  SVEDA_APPEARANCE_PRESET_IDS,
  SVEDA_APPEARANCE_PRESETS,
  SVEDA_APPEARANCE_STYLE_ID,
} from '../src/appearance.js';

afterEach(() => {
  document.getElementById(SVEDA_APPEARANCE_STYLE_ID)?.remove();
});

describe('SVEDA_APPEARANCE_PRESETS', () => {
  it('keeps the landing/playground ink look as default', () => {
    const preset = SVEDA_APPEARANCE_PRESETS.default;

    expect(preset.preset).toBe('default');
    expect(preset.radius).toBe('0px');
    expect(preset.tokens.background).toBe('0 0% 100%');
    expect(preset.tokens.foreground).toBe('0 0% 4%');
    expect(preset.tokens.brand).toBe('0 0% 4%');
    expect(preset.tokens.border).toBe('42 18% 82%');
  });

  it('gives every named preset a distinct brand color', () => {
    const brands = SVEDA_APPEARANCE_PRESET_IDS.map((id) => SVEDA_APPEARANCE_PRESETS[id].tokens.brand);

    expect(brands).toHaveLength(6);
    expect(new Set(brands).size).toBe(6);
    expect(SVEDA_APPEARANCE_PRESETS.lms.tokens.brand).toBe('275 96% 52%');
    expect(SVEDA_APPEARANCE_PRESETS.ocean.tokens.brand).toBe('221 83% 53%');
  });
});

describe('resolveSvedaAppearance', () => {
  it('resolves an empty payload to the default ink preset', () => {
    const resolved = resolveSvedaAppearance({});

    expect(resolved?.preset).toBe('default');
    expect(resolved?.radius).toBe('0px');
    expect(resolved?.tokens).toEqual(SVEDA_APPEARANCE_PRESETS.default.tokens);
  });

  it('resolves a named preset without copying host tokens', () => {
    const resolved = resolveSvedaAppearance({ preset: 'lms', radius: '8px' });

    expect(resolved?.preset).toBe('lms');
    expect(resolved?.radius).toBe('8px');
    expect(resolved?.tokens?.brand).toBe('275 96% 52%');
  });
});

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
        { preset: 'default', radius: '0px', launcher: { label: 'ADMIN' }, chrome: { modelSelect: true } },
        { preset: 'ocean', launcher: { label: 'HELP' } },
      ),
    ).toEqual({
      preset: 'ocean',
      radius: '0px',
      launcher: { label: 'HELP' },
      chrome: { modelSelect: true },
    });
  });

  it('drops admin tokens when the host sets a named preset', () => {
    expect(
      mergeSvedaAppearance({ preset: 'sand', tokens: { brand: '30 20% 20%' } }, { preset: 'lms' }),
    ).toEqual({ preset: 'lms' });
  });

  it('overlays host tokens onto an admin preset when the host does not rename it', () => {
    expect(
      mergeSvedaAppearance({ preset: 'default', tokens: { brand: '0 0% 4%' } }, { tokens: { brand: '12 80% 50%' } }),
    ).toEqual({
      preset: 'default',
      tokens: { brand: '12 80% 50%' },
    });
  });
});

describe('applySvedaAppearance', () => {
  it('writes default ink tokens for an empty appearance', () => {
    applySvedaAppearance({});
    const css = document.getElementById(SVEDA_APPEARANCE_STYLE_ID)?.textContent ?? '';

    expect(css).toContain('--sveda-radius:0px');
    expect(css).toContain('--sveda-brand:0 0% 4%');
    expect(buildAppearanceCss({})).toContain('--sveda-brand:0 0% 4%');
  });

  it('writes the LMS brand when that preset is applied', () => {
    applySvedaAppearance({ preset: 'lms' });
    const css = document.getElementById(SVEDA_APPEARANCE_STYLE_ID)?.textContent ?? '';

    expect(css).toContain('--sveda-brand:275 96% 52%');
  });
});
