import DOMPurify from 'dompurify';
import hljs from 'highlight.js';
import { Marked, Renderer } from 'marked';
import type { VedaResourceLink } from './assistantMessage.js';

const escapeHtml = (value: string): string =>
  value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');

const renderer = new Renderer();
renderer.code = ({ text, lang }): string => {
  const language = lang && hljs.getLanguage(lang) ? lang : '';
  const value = language ? hljs.highlight(text, { language }).value : escapeHtml(text);
  const className = language ? ` class="hljs language-${language}"` : '';
  return `<pre><code${className}>${value}</code></pre>`;
};

const markedInstance = new Marked({ gfm: true, breaks: true });
markedInstance.use({ renderer });

export function injectResourceLinks(html: string, resources: VedaResourceLink[]): string {
  if (!Array.isArray(resources) || resources.length === 0) {
    return html;
  }

  const sorted = [...resources].sort((a, b) => b.label.length - a.label.length);

  for (const resource of sorted) {
    if (!resource.label || !resource.url) {
      continue;
    }
    const escaped = resource.label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const pattern = new RegExp(
      `(?<=^|[^a-zA-Zа-яА-ЯёЁ])(«${escaped}»|„${escaped}"|"${escaped}"|«${escaped}|${escaped}»|${escaped})(?=$|[^a-zA-Zа-яА-ЯёЁ])`,
      'gu'
    );
    const safeLink = resource.url.replace(/"/g, '&quot;');
    const icon =
      '<svg class="resource-link-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 7h10v10"/><path d="M7 17 17 7"/></svg>';
    html = html.replace(pattern, `<a href="${safeLink}" class="resource-inline-link">$1${icon}</a>`);
  }

  return html;
}

export interface VedaMarkdownOptions {
  resources?: VedaResourceLink[];
  injectResources?: boolean;
}

export function renderMarkdown(text: string, options: VedaMarkdownOptions = {}): string {
  const body = text || '';
  if (!body.trim()) {
    return '';
  }

  const raw = markedInstance.parse(body, { async: false });
  let html = DOMPurify.sanitize(raw, {
    ALLOWED_TAGS: [
      'p',
      'br',
      'strong',
      'em',
      'code',
      'pre',
      'ul',
      'ol',
      'li',
      'h1',
      'h2',
      'h3',
      'h4',
      'a',
      'blockquote',
      'hr',
      'span',
      'img',
    ],
    ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class', 'target', 'rel', 'loading'],
  });

  if (options.injectResources && Array.isArray(options.resources) && options.resources.length > 0) {
    html = injectResourceLinks(html, options.resources);
  }

  return html;
}
