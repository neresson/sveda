<script setup lang="ts">
import { cn } from '../../lib/utils.js'
import { ToastViewport, type ToastViewportProps } from 'reka-ui'
import { computed, type HTMLAttributes } from 'vue'

const props = defineProps<ToastViewportProps & { class?: HTMLAttributes['class'] }>()

const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props

  return delegated
})
</script>

<template>
  <ToastViewport v-bind="delegatedProps" :class="cn('sveda-chat pointer-events-none absolute inset-x-0 bottom-0 top-auto z-[100] flex h-auto max-h-48 w-full flex-col justify-end gap-2 p-3', props.class)" />
</template>
