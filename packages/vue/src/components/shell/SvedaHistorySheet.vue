<script setup>
  import ChatHistoryDropdown from '../chat/ChatHistoryDropdown.vue';
  import { Button } from '../../ui/button';
  import { Sheet, SheetContent, SheetHeader, SheetTitle } from '../../ui/sheet';
  import { Plus } from 'lucide-vue-next';

  const open = defineModel('open', { type: Boolean, default: false });

  defineProps({
    title: { type: String, required: true },
    histories: { type: Array, required: true },
    currentChatId: { type: String, default: null },
    newChatLabel: { type: String, required: true },
  });

  const emit = defineEmits(['new-chat', 'select-chat', 'delete-chat', 'rename-chat']);
</script>

<template>
  <Sheet v-model:open="open">
    <SheetContent
      side="bottom"
      class="flex h-[85vh] max-h-[85vh] flex-col overflow-hidden rounded-t-[var(--sveda-radius)] p-0"
    >
      <SheetHeader class="border-b border-border/50 p-4 pr-12 text-left">
        <SheetTitle>{{ title }}</SheetTitle>
      </SheetHeader>
      <div class="border-b border-border/50 p-3">
        <Button
          variant=""
          class="w-full justify-center gap-2"
          @click="emit('new-chat')"
        >
          <Plus class="h-4 w-4" />
          {{ newChatLabel }}
        </Button>
      </div>
      <div class="min-h-0 flex-1">
        <ChatHistoryDropdown
          variant="sidebar"
          :show="true"
          :histories="histories"
          :current-chat-id="currentChatId"
          @close="open = false"
          @select-chat="emit('select-chat', $event)"
          @delete-chat="emit('delete-chat', $event)"
          @rename-chat="emit('rename-chat', $event)"
        />
      </div>
    </SheetContent>
  </Sheet>
</template>
