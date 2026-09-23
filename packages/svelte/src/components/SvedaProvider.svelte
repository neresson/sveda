<script lang="ts">
  import type { Snippet } from 'svelte';
  import { untrack } from 'svelte';
  import {
    createSveda,
    type SvedaContext,
    type SvedaPluginOptions,
  } from '../provider.js';
  import { setSvedaContext } from '../context.js';

  interface Props {
    options?: SvedaPluginOptions;
    sveda?: SvedaContext;
    children?: Snippet;
  }

  let { options, sveda, children }: Props = $props();

  // Context is fixed for the provider lifetime (mirrors Vue app.use(createSveda(...))).
  const ctx = untrack(() => sveda ?? (options ? createSveda(options) : null));
  if (!ctx) {
    throw new Error('[sveda] SvedaProvider requires either `sveda` or `options`.');
  }
  setSvedaContext(ctx);
</script>

{#if children}
  {@render children()}
{/if}
