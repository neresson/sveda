<script setup lang="ts">
import { isVNode } from 'vue'
import {
  Toast,
  ToastClose,
  ToastDescription,
  ToastProvider,
  ToastTitle,
  ToastViewport,
  useToast,
} from './index.js'

const { toasts } = useToast()

const isStringDescription = (value: unknown): value is string => typeof value === 'string'
const isRenderableDescription = (value: unknown) => isVNode(value) || typeof value === 'function'
</script>

<template>
  <ToastProvider>
    <Toast
      v-for="toastItem in toasts"
      :key="toastItem.id"
      v-bind="toastItem"
    >
      <div class="grid gap-1">
        <ToastTitle v-if="toastItem.title">
          {{ toastItem.title }}
        </ToastTitle>
        <ToastDescription v-if="toastItem.description">
          <template v-if="isStringDescription(toastItem.description)">
            {{ toastItem.description }}
          </template>
          <component
            :is="toastItem.description"
            v-else-if="isRenderableDescription(toastItem.description)"
          />
        </ToastDescription>
      </div>
      <component
        :is="toastItem.action"
        v-if="toastItem.action"
      />
      <ToastClose />
    </Toast>
    <ToastViewport />
  </ToastProvider>
</template>
