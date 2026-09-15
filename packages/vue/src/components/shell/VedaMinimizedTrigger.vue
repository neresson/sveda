<script setup>
  import { Button } from '../../ui/button';
  import { computed } from 'vue';
  import { vedaLauncherIconComponent } from '../../launcherIcons';

  const props = defineProps({
    label: { type: String, required: true },
    icon: { type: String, default: 'sparkles' },
    logoSrc: { type: String, default: '' },
    logoAlt: { type: String, default: '' },
  });

  defineEmits(['open']);

  const Icon = computed(() => vedaLauncherIconComponent(props.icon));

  const triggerClass =
    'flex h-10 min-w-[200px] items-center justify-start gap-2 rounded-bl-none rounded-br-none rounded-tl-[var(--veda-radius)] rounded-tr-none bg-primary px-3 text-primary-foreground shadow-2xl transition-all hover:bg-primary/90 min-[768px]:min-w-0 min-[1872px]:h-14 min-[1872px]:justify-center min-[1872px]:gap-3 min-[1872px]:rounded-[var(--veda-radius)] min-[1872px]:px-4 [&_svg]:!h-6 [&_svg]:!w-6 min-[1872px]:[&_svg]:!h-10 min-[1872px]:[&_svg]:!w-10';
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
      class="font-semibold"
    >{{ label }}</span>
  </Button>
</template>
