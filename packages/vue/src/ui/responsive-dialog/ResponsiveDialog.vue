<script setup>
import { useMediaQuery } from '@vueuse/core';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '../dialog/index.js';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetFooter } from '../sheet/index.js';

defineProps({
  open: Boolean,
  contentClass: String,
});

const emit = defineEmits(['update:open']);

const isMobile = useMediaQuery('(max-width: 639px)');

const updateOpen = (value) => {
  emit('update:open', value);
};
</script>

<template>
  <Sheet v-if="isMobile" :open="open" @update:open="updateOpen">
    <SheetContent side="bottom" class="rounded-t-[var(--veda-radius)] max-h-[90vh] overflow-y-auto">
      <SheetHeader>
        <SheetTitle>
          <slot name="title" />
        </SheetTitle>
      </SheetHeader>
      <div v-if="$slots.default" class="py-4">
        <slot />
      </div>
      <div v-else class="py-4 text-muted-foreground text-sm">
        <slot name="description" />
      </div>
      <SheetFooter v-if="$slots.footer" class="gap-2">
        <slot name="footer" />
      </SheetFooter>
    </SheetContent>
  </Sheet>
  <Dialog v-else :open="open" @update:open="updateOpen">
    <DialogContent :class="contentClass || ''">
      <DialogHeader>
        <DialogTitle>
          <slot name="title" />
        </DialogTitle>
        <DialogDescription v-if="!$slots.default">
          <slot name="description" />
        </DialogDescription>
      </DialogHeader>
      <slot v-if="$slots.default" />
      <DialogFooter v-if="$slots.footer">
        <slot name="footer" />
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
