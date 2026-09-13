<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { ref } from 'vue'
import { cn } from '../../lib/utils.js'
import { Primitive, type PrimitiveProps } from 'reka-ui'
import { type ButtonVariants, buttonVariants, buttonHoverStyles } from './index.js'

interface Props extends PrimitiveProps {
  variant?: ButtonVariants['variant']
  size?: ButtonVariants['size']
  class?: HTMLAttributes['class']
}

const props = withDefaults(defineProps<Props>(), {
  as: 'button',
})

const isHovered = ref<boolean>(false);

const handleTouchStart = (): void => {
  isHovered.value = true;
};

const handleTouchEnd = (): void => {
  setTimeout(() => {
    isHovered.value = false;
  }, 150);
};

const handleMouseEnter = (): void => {
  isHovered.value = true;
};

const handleMouseLeave = (): void => {
  isHovered.value = false;
};
</script>

<template>
  <Primitive
    :as="as"
    :as-child="asChild"
    :class="
        cn(
            buttonVariants({ variant, size }),
            { [buttonHoverStyles[variant ?? 'default']]: isHovered },
            props.class
        )"
    @touchstart.passive="handleTouchStart"
    @touchend.passive="handleTouchEnd"
    @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave"
  >
    <slot />
  </Primitive>
</template>
