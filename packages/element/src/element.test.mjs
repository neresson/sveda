import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import { parseSessionAttributes } from './session.ts';
import { buildSvedaChatCss, scopeEmbedCssForHost } from './scope-embed-css.mjs';

describe('host stylesheet', () => {
  it('sizes the custom element as the floating window instead of the host viewport', async () => {
    const css = await readFile(new URL('./host.css', import.meta.url), 'utf8');

    assert.match(css, /position:\s*fixed/);
    assert.match(css, /--sveda-embed-width/);
    assert.match(css, /--sveda-embed-height/);
    assert.match(css, /data-sveda-open/);
    assert.match(css, /pointer-events:\s*none/);
    assert.match(css, /:where\(sveda-chat\) button/);
    assert.match(css, /background-color:\s*transparent/);
    assert.match(css, /font:\s*inherit/);
    assert.equal(/sveda-chat\s*\{[^}]*\binset\b/.test(css), false);
    assert.equal(/sveda-chat\s*\{[^}]*top:\s*0/.test(css), false);
    assert.match(css, /sveda-chat\[data-sveda-open='true'\]\[data-sveda-immersive='true'\]/);
    assert.match(css, /sveda-chat\[data-sveda-open='true'\]\[data-sveda-fixed='true'\]/);
    assert.match(css, /sveda-chat\[data-sveda-open='true'\] > \.sveda-chat/);
    assert.match(css, /0 0% 100%/);
    assert.match(css, /border:\s*1px dashed/);
    assert.match(css, /background-size:\s*10px 1px/);
    assert.match(css, /ol\.pointer-events-none/);
    assert.match(css, /max-height:\s*12rem/);
    assert.equal(css.includes("sveda-chat[data-sveda-open='true'] .sveda-chat"), false);
  });

  it('does not let the toast viewport swallow clicks', async () => {
    const viewport = await readFile(
      new URL('../../vue/src/ui/toast/ToastViewport.vue', import.meta.url),
      'utf8'
    );

    assert.match(viewport, /pointer-events-none/);
    assert.match(viewport, /absolute inset-x-0 bottom-0/);
    assert.match(viewport, /max-h-48/);
    assert.equal(viewport.includes('max-h-screen'), false);
    assert.equal(viewport.includes('main-sm:'), false);
  });

  it('surfaces stream error payloads instead of a generic toast', async () => {
    const page = await readFile(
      new URL('../../vue/src/composables/useSvedaChatPage.ts', import.meta.url),
      'utf8'
    );

    assert.match(page, /export const svedaErrorMessage/);
    assert.match(page, /svedaErrorMessage\(error, t\('errorSendingMessage'\)\)/);
  });

  it('reuses iframe embed.css without a second Tailwind pipeline', async () => {
    const pkg = await readFile(new URL('../package.json', import.meta.url), 'utf8');
    const index = await readFile(new URL('./index.ts', import.meta.url), 'utf8');

    assert.equal(pkg.includes('@tailwindcss/vite'), false);
    assert.equal(pkg.includes('"tailwindcss"'), false);
    assert.match(index, /import '\.\/host\.css'/);
    assert.match(index, /static get observedAttributes/);
    assert.match(index, /attributeChangedCallback/);
  });

  it('scopes iframe embed CSS so preflight cannot reset the host page', () => {
    const host = 'sveda-chat{position:fixed}';
    const css = buildSvedaChatCss(
      host,
      '@import "https://fonts.example/css2?family=Geist:wght@400;500";@layer theme{:root,:host{--font-sans:Geist}}@layer base{*{margin:0;padding:0}button{background:#000}}@layer utilities{.flex{display:flex}}@layer properties{@supports (color:red){*,:before,:after,::backdrop{--tw-translate-x:0}}}*{scrollbar-width:thin;scrollbar-color:var(--color-muted) transparent}::-webkit-scrollbar{width:6px}html,.overflow-auto,.overflow-y-auto,.overflow-y-scroll,.overflow-scroll,textarea{scrollbar-gutter:stable}',
    );

    assert.equal(css.indexOf('@import'), 0);
    assert.match(css, /@import "https:\/\/fonts\.example\/css2\?family=Geist:wght@400;500";/);
    assert.match(css, /@layer properties,theme,base,components,utilities;/);
    assert.match(css, /@layer base\{sveda-chat\{position:fixed\}\}/);
    assert.match(css, /@layer theme\{:root,:host\{/);
    assert.match(css, /\.flex\{display:flex\}/);
    assert.match(css, /sveda-chat,sveda-chat \*,sveda-chat :before/);
    assert.equal(css.includes('*{margin:0'), false);
    assert.equal(css.includes('button{background:#000}'), false);
    assert.match(css, /sveda-chat \*,\.sveda-chat \*\{scrollbar-width:thin/);
    assert.match(css, /sveda-chat ::-webkit-scrollbar,\.sveda-chat ::-webkit-scrollbar/);
    assert.match(css, /sveda-chat,\.sveda-chat,\.sveda-chat \.overflow-auto/);
  });

  it('strips iframe preflight from the built embed stylesheet', async () => {
    const embedPath = new URL('../../../apps/runtime/public/build/sveda/embed.css', import.meta.url);
    const embed = await readFile(embedPath, 'utf8');
    const scoped = scopeEmbedCssForHost(embed);

    assert.match(embed, /@layer base\{/);
    assert.equal(scoped.includes('@layer base'), false);
    assert.match(scoped, /@layer theme\{:root,:host\{/);
    assert.match(scoped, /\.flex\{display:flex\}/);
    assert.equal(scoped.includes('*{margin:0'), false);
  });

  it('makes the custom element the chat window', async () => {
    const element = await readFile(new URL('./SvedaChatElement.vue', import.meta.url), 'utf8');
    const chat = await readFile(new URL('../../vue/src/components/shell/SvedaChat.vue', import.meta.url), 'utf8');

    assert.match(element, /provide\(SvedaFillHostKey, true\)/);
    assert.match(element, /relative flex h-full min-h-0 w-full flex-col/);
    assert.match(chat, /chat\.fillHost/);
    assert.match(chat, /relative h-full min-h-0 w-full overflow-hidden/);
    assert.match(chat, /SvedaResizeHandles/);
    assert.match(
      chat,
      /<\/Card>\s*<SvedaFrameTicks[\s\S]*<SvedaResizeHandles/,
    );
  });

  it('expands the iframe host for history and fullscreen', async () => {
    const widget = await readFile(
      new URL('../../../apps/runtime/resources/embed/widget.js', import.meta.url),
      'utf8'
    );
    const layout = await readFile(
      new URL('../../vue/src/composables/useSvedaChatLayout.ts', import.meta.url),
      'utf8'
    );
    const history = await readFile(
      new URL('../../vue/src/components/chat/ChatHistoryDropdown.vue', import.meta.url),
      'utf8'
    );

    assert.match(widget, /applyFullscreenLayout/);
    assert.match(widget, /applyFixedLayout/);
    assert.match(widget, /data\.immersive/);
    assert.match(widget, /data\.fixed/);
    assert.match(layout, /historySidebarVisible/);
    assert.match(layout, /data-sveda-immersive/);
    assert.match(history, /flex h-10 items-center gap-2/);
    assert.equal(history.includes('absolute left-3 top-1/2'), false);
  });
});

describe('parseSessionAttributes', () => {
  it('reads session URL from element attributes', () => {
    const element = {
      getAttribute(name) {
        if (name === 'session') {
          return '/sveda/session';
        }

        return null;
      },
    };

    assert.deepEqual(parseSessionAttributes(element), {
      session: '/sveda/session',
      origin: null,
      token: null,
    });
  });

  it('reads origin and token attributes', () => {
    const element = {
      getAttribute(name) {
        if (name === 'origin') {
          return 'http://127.0.0.1:8787';
        }
        if (name === 'token') {
          return 'sveda_embed_test';
        }

        return null;
      },
    };

    assert.deepEqual(parseSessionAttributes(element), {
      session: null,
      origin: 'http://127.0.0.1:8787',
      token: 'sveda_embed_test',
    });
  });
});
