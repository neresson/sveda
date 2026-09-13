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
  <ToastViewport v-bind="delegatedProps" :class="cn('fixed top-0 z-[100] flex max-h-screen w-full flex-col-reverse p-4 pt-[max(1rem,env(safe-area-inset-top))] main-sm:bottom-0 main-sm:left-0 main-sm:top-auto main-sm:flex-col main-md:max-w-[420px]', props.class)" />
</template>
