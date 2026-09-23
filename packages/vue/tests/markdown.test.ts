import { describe, expect, it } from 'vitest';
import { renderMarkdown } from '../src/lib/markdown';

const sample = `То есть это скорее справочник «всё о воде» — хорошая отправная точка.

## 🌍 Альтернатива — «Water»
https://wmo.int/themes/water

Более прикладная статья про **круговорот воды**.

---

### Уточни, что именно нужно
- Наука/химия — почему вода такое необычное вещество
- Питьевая вода — нормы, качество
`;

describe('renderMarkdown', () => {
  it('turns headings, lists, rules and emphasis into HTML instead of leftover markup', () => {
    const html = renderMarkdown(sample);

    expect(html).toContain('<h2>');
    expect(html).toContain('<h3>');
    expect(html).toContain('<ul>');
    expect(html).toContain('<li>');
    expect(html).toContain('<hr>');
    expect(html).toContain('<strong>круговорот воды</strong>');
    expect(html).not.toContain('## ');
    expect(html).not.toContain('### ');
    expect(html).not.toMatch(/<p>---<\/p>/);
    expect(html).not.toContain('- Наука/химия');
  });

  it('keeps links clickable', () => {
    const html = renderMarkdown('See [docs](https://example.com/path).');
    expect(html).toContain('href="https://example.com/path"');
    expect(html).not.toContain('[docs]');
  });
});
