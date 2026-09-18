<script setup>
  import { Button } from '../../ui/button';
  import { computed } from 'vue';
  import { svedaLauncherIconComponent } from '../../launcherIcons';

  const props = defineProps({
    label: { type: String, required: true },
    icon: { type: String, default: 'sparkles' },
    logoSrc: { type: String, default: '' },
    logoAlt: { type: String, default: '' },
  });

  defineEmits(['open']);

  const Icon = computed(() => svedaLauncherIconComponent(props.icon));

  const triggerClass =
    'inline-flex h-auto min-w-0 items-center justify-center gap-2 rounded-none border border-primary bg-primary px-3.5 py-2 font-mono text-[11px] tracking-[0.14em] text-primary-foreground shadow-none transition-opacity hover:opacity-90 [&_svg]:!h-3.5 [&_svg]:!w-3.5';
</script>

<template>
  <Button
    size="lg"
    :class="triggerClass"
    @click="$emit('open')"
  >
    <img
      v-if="logoSrc"
      :src="logoSrc"
      :alt="logoAlt"
      class="h-6 w-6 flex-shrink-0 object-cover min-[1872px]:h-10 min-[1872px]:w-10"
    />
    <component
      :is="Icon"
      v-else
      class="h-6 w-6 flex-shrink-0"
    />
    <span
      v-if="label"
      class="font-mono text-[11px] font-normal tracking-[0.14em]"
    >{{ label }}</span>
  </Button>
</template>
