import { describe, expect, it } from 'vitest';
import { VedaContextRegistry } from '../src/context.js';

describe('VedaContextRegistry', () => {
  it('snapshots static values keyed by description', () => {
    const registry = new VedaContextRegistry();
    registry.register('current page', { type: 'course', id: 7 });

    expect(registry.snapshot()).toEqual({
      'current page': { type: 'course', id: 7 },
    });
  });

  it('evaluates function values at snapshot time', () => {
    const registry = new VedaContextRegistry();
    let counter = 0;
    registry.register('counter', () => ++counter);

    expect(registry.snapshot()).toEqual({ counter: 1 });
    expect(registry.snapshot()).toEqual({ counter: 2 });
  });

  it('unregisters via returned disposer', () => {
    const registry = new VedaContextRegistry();
    const dispose = registry.register('temp', 1);
    dispose();

    expect(registry.snapshot()).toEqual({});
  });

  it('overrides duplicate descriptions', () => {
    const registry = new VedaContextRegistry();
    registry.register('page', 'a');
    registry.register('page', 'b');

    expect(registry.snapshot()).toEqual({ page: 'b' });
  });
});
