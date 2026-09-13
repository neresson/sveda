import type { Component, ComputedRef, VNode } from 'vue'
import { computed, shallowRef } from 'vue'
import type { ToastProps } from './index.js'

const TOAST_LIMIT = 3
const TOAST_REMOVE_DELAY = 5000

export type StringOrVNode =
  | string
  | VNode
  | (() => VNode)

export type ToasterToast = ToastProps & {
  id: string
  title?: string
  description?: StringOrVNode
  action?: Component
}

let count = 0

function genId(): string {
  count = (count + 1) % Number.MAX_SAFE_INTEGER
  return count.toString()
}

interface ToastState {
  toasts: ToasterToast[]
}

const state = shallowRef<ToastState>({ toasts: [] })
const toastTimeouts = new Map<string, ReturnType<typeof setTimeout>>()

function removeToast(toastId: string): void {
  state.value.toasts = state.value.toasts.filter(toast => toast.id !== toastId)
}

function addToRemoveQueue(toastId: string): void {
  if (toastTimeouts.has(toastId)) {
    return
  }

  const timeout = setTimeout(() => {
    toastTimeouts.delete(toastId)
    removeToast(toastId)
  }, TOAST_REMOVE_DELAY)

  toastTimeouts.set(toastId, timeout)
}

function dismiss(toastId?: string): void {
  if (toastId) {
    addToRemoveQueue(toastId)
    state.value.toasts = state.value.toasts.map(toast =>
      toast.id === toastId ? { ...toast, open: false } : toast
    )
    return
  }

  for (const toast of state.value.toasts) {
    addToRemoveQueue(toast.id)
  }
  state.value.toasts = state.value.toasts.map(toast => ({ ...toast, open: false }))
}

export type ToastOptions = Omit<ToasterToast, 'id'>

function toast(props: ToastOptions) {
  const id = genId()

  const update = (next: ToastOptions) => {
    state.value.toasts = state.value.toasts.map(toast =>
      toast.id === id ? { ...toast, ...next, id } : toast
    )
  }

  const onOpenChange = (open: boolean) => {
    if (!open) {
      dismiss(id)
    }
  }

  state.value.toasts = [
    { ...props, id, open: true, onOpenChange },
    ...state.value.toasts,
  ].slice(0, TOAST_LIMIT)

  return {
    id,
    dismiss: () => dismiss(id),
    update,
  }
}

export interface UseToastReturn {
  toasts: ComputedRef<ToasterToast[]>
  toast: typeof toast
  dismiss: (toastId?: string) => void
}

function useToast(): UseToastReturn {
  return {
    toasts: computed(() => state.value.toasts),
    toast,
    dismiss,
  }
}

export { toast, useToast }
