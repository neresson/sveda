const IMPORT_STATEMENT = /@import\s+(?:url\(\s*)?(?:"[^"]*"|'[^']*')\s*\)?[^;]*;/g;

export const removeAtLayerBlock = (css, name) => {
  const needle = `@layer ${name}{`;
  const start = css.indexOf(needle);
  if (start < 0) {
    return css;
  }

  let index = start + needle.length;
  let depth = 1;
  while (index < css.length && depth > 0) {
    const char = css[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
    }
    index += 1;
  }

  return css.slice(0, start) + css.slice(index);
};

export const scopeEmbedCssForHost = (embedCss) => {
  let css = embedCss.replace(IMPORT_STATEMENT, '');
  css = removeAtLayerBlock(css, 'base');
  css = css.replace(
    /\*,:before,:after,::backdrop\{--tw-translate-x:0/g,
    'sveda-chat,sveda-chat *,sveda-chat :before,sveda-chat :after,.sveda-chat,.sveda-chat *,.sveda-chat :before,.sveda-chat :after,:host{--tw-translate-x:0',
  );
  css = css.replace(
    /\*\{scrollbar-width:thin;scrollbar-color:var\(--color-muted\) transparent\}/g,
    'sveda-chat *,.sveda-chat *{scrollbar-width:thin;scrollbar-color:var(--color-muted) transparent}',
  );
  css = css.replace(/::-webkit-scrollbar/g, 'sveda-chat ::-webkit-scrollbar,.sveda-chat ::-webkit-scrollbar');
  css = css.replace(
    /html,\.overflow-auto,\.overflow-y-auto,\.overflow-y-scroll,\.overflow-scroll,textarea\{scrollbar-gutter:stable\}/g,
    'sveda-chat,.sveda-chat,.sveda-chat .overflow-auto,.sveda-chat .overflow-y-auto,.sveda-chat .overflow-y-scroll,.sveda-chat .overflow-scroll,.sveda-chat textarea{scrollbar-gutter:stable}',
  );

  return css;
};

export const buildSvedaChatCss = (hostCss, embedCss) => {
  const imports = [];
  embedCss.replace(IMPORT_STATEMENT, (statement) => {
    imports.push(statement);
    return '';
  });

  const scoped = scopeEmbedCssForHost(embedCss);
  const prelude = [
    ...imports,
    '@layer sveda-host,properties,theme,components,utilities;',
    `@layer sveda-host{${hostCss.trim()}}`,
  ].join('');

  return `${prelude}${scoped}`;
};
