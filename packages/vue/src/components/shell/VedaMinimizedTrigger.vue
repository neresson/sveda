<script setup>
  import { Button } from '../../ui/button';
  import { Sparkles } from 'lucide-vue-next';

  defineProps({
    label: { type: String, required: true },
    logoSrc: { type: String, default: '' },
    logoAlt: { type: String, default: '' },
  });

  defineEmits(['open']);

  const triggerClass =
    'flex h-10 min-w-[200px] items-center justify-start gap-2 rounded-bl-none rounded-br-none rounded-tl-2xl rounded-tr-none bg-primary px-3 text-primary-foreground shadow-2xl transition-all hover:bg-primary/90 min-[768px]:min-w-0 min-[1872px]:h-14 min-[1872px]:justify-center min-[1872px]:gap-3 min-[1872px]:rounded-full min-[1872px]:px-4 [&_svg]:!h-6 [&_svg]:!w-6 min-[1872px]:[&_svg]:!h-10 min-[1872px]:[&_svg]:!w-10';
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
      class="h-6 w-6 flex-shrink-0 min-[1872px]:h-10 min-[1872px]:w-10"
    />
    <Sparkles
      v-else
      class="h-6 w-6 flex-shrink-0"
    />
    <span class="font-semibold">{{ label }}</span>
  </Button>
</template>
