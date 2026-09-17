<script setup>
  import ChatHistoryDropdown from '../chat/ChatHistoryDropdown.vue';
  import { Button } from '../../ui/button';
  import { Plus } from 'lucide-vue-next';

  defineProps({
    surfaceClass: { type: String, required: true },
    histories: { type: Array, required: true },
    currentChatId: { type: String, default: null },
    newChatLabel: { type: String, required: true },
  });

  const emit = defineEmits(['new-chat', 'select-chat', 'delete-chat', 'rename-chat', 'close']);
</script>

<template>
  <aside :class="surfaceClass">
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
        @close="emit('close')"
        @select-chat="emit('select-chat', $event)"
        @delete-chat="emit('delete-chat', $event)"
        @rename-chat="emit('rename-chat', $event)"
      />
    </div>
  </aside>
</template>
