<script setup lang="ts">
import { cn } from '../../lib/utils.js'
import {
  SwitchRoot,
  type SwitchRootEmits,
  type SwitchRootProps,
  SwitchThumb,
  useForwardPropsEmits,
} from 'reka-ui'
import { computed, type HTMLAttributes } from 'vue'

const props = defineProps<SwitchRootProps & { class?: HTMLAttributes['class']; size?: 'default' | 'sm' }>()

const emits = defineEmits<SwitchRootEmits>()

const delegatedProps = computed(() => {
  const { class: _, size: __, ...delegated } = props

  return delegated
})

const forwarded = useForwardPropsEmits(delegatedProps, emits)

const sizeClasses = computed(() =>
  props.size === 'sm'
    ? {
        root: 'h-5 w-9',
        thumb: 'h-4 w-4 data-[state=checked]:translate-x-4',
      }
    : {
        root: 'h-6 w-11',
        thumb: 'h-5 w-5 data-[state=checked]:translate-x-5',
      },
)
</script>

<template>
  <SwitchRoot
    v-bind="forwarded"
    :class="cn(
      'peer inline-flex shrink-0 cursor-pointer items-center rounded-full p-0.5 shadow-inner transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-card disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-brand-primary-purple data-[state=checked]:shadow-[inset_0_1px_2px_rgba(0,0,0,0.22),0_1px_8px_-2px_rgba(0,0,0,0.35)] data-[state=unchecked]:bg-muted-foreground/35 data-[state=unchecked]:shadow-[inset_0_1px_3px_rgba(0,0,0,0.28)] dark:data-[state=unchecked]:bg-white/25 dark:data-[state=unchecked]:shadow-[inset_0_1px_3px_rgba(0,0,0,0.5)]',
      sizeClasses.root,
      props.class,
    )"
  >
    <SwitchThumb
      :class="cn(
        'pointer-events-none block rounded-full bg-brand-primary-purple-foreground shadow-[0_1px_2px_rgba(0,0,0,0.28),0_2px_6px_rgba(0,0,0,0.18)] ring-0 transition-transform data-[state=unchecked]:translate-x-0',
        sizeClasses.thumb,
      )"
    >
      <slot name="thumb" />
    </SwitchThumb>
  </SwitchRoot>
</template>
