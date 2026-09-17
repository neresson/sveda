<script setup>
  import { Button } from '../../ui/button';
  import { ScrollArea, ScrollBar } from '../../ui/scroll-area';
  import { buildChatTabTitleMap } from '../../lib/chatTabs';
  import { Loader2, Plus, X } from 'lucide-vue-next';
  import { computed, unref } from 'vue';
  import { useSvedaT } from '../../i18n/index';

  const t = useSvedaT();

  const props = defineProps({
    tabs: {
      type: Array,
      required: true,
    },
    currentChatId: {
      type: String,
      default: null,
    },
    streamingChatIds: {
      type: Object,
      default: () => new Set(),
    },
    unreadChatIds: {
      type: Object,
      default: () => new Set(),
    },
    compact: {
      type: Boolean,
      default: false,
    },
  });

  const emit = defineEmits(['select-chat', 'close-tab', 'new-chat']);

  const visibleTabs = computed(() => props.tabs.filter(tab => tab?.id));

  const tabTitles = computed(() => buildChatTabTitleMap(visibleTabs.value, t));

  const resolveIdSet = value => {
    const resolved = unref(value);
    if (resolved instanceof Set) {
      return resolved;
    }
    return new Set();
  };

  const isStreaming = chatId => resolveIdSet(props.streamingChatIds).has(chatId);

  const hasUnread = chatId => !isStreaming(chatId) && resolveIdSet(props.unreadChatIds).has(chatId);

  const tabTriggerClass = chatId =>
    [
      'inline-flex h-10 max-w-56 shrink-0 items-center gap-1.5 whitespace-nowrap border-b-2 border-transparent px-4 py-2 text-sm font-medium text-foreground/70 transition-all hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
      props.currentChatId === chatId
        ? 'border-primary bg-muted/60 text-foreground shadow-sm'
        : '',
    ].join(' ');

  const closeTab = (chatId, event) => {
    event.stopPropagation();
    emit('close-tab', chatId);
  };
</script>

<template>
  <div
    v-if="visibleTabs.length > 0"
    class="flex shrink-0 items-stretch border-b border-border/50 bg-background/70"
    :class="compact ? 'px-3' : 'px-4 md:px-6'"
  >
    <ScrollArea class="min-w-0 flex-1">
      <div class="flex h-12 min-w-max items-end gap-px px-1">
        <div
          v-for="tab in visibleTabs"
          :key="tab.id"
          role="button"
          tabindex="0"
          :class="tabTriggerClass(tab.id)"
          @click="emit('select-chat', tab.id)"
          @keydown.enter.prevent="emit('select-chat', tab.id)"
          @keydown.space.prevent="emit('select-chat', tab.id)"
        >
          <Loader2
            v-if="isStreaming(tab.id)"
            class="h-3.5 w-3.5 shrink-0 animate-spin text-primary"
            :aria-label="t('chatTabRunning')"
          />
          <span
            v-else-if="hasUnread(tab.id)"
            class="h-2 w-2 shrink-0 rounded-full bg-primary"
            :aria-label="t('chatTabUnread')"
          />
          <span class="min-w-0 truncate">
            {{ tabTitles.get(tab.id) }}
          </span>
          <button
            type="button"
            class="ml-0.5 rounded-sm p-0.5 text-muted-foreground opacity-70 hover:bg-background/80 hover:text-foreground hover:opacity-100"
            :aria-label="t('closeChatTab')"
            @mousedown.stop
            @click.stop="closeTab(tab.id, $event)"
          >
            <X class="h-3 w-3" />
          </button>
        </div>
      </div>
      <ScrollBar orientation="horizontal" />
    </ScrollArea>

    <div class="flex shrink-0 items-center border-l border-border/40 pl-1">
      <Button
        variant="ghostTransparent"
        size="icon"
        class="h-9 w-9 shrink-0 text-muted-foreground hover:text-foreground"
        :aria-label="t('newChat')"
        @click="emit('new-chat')"
      >
        <Plus class="h-4 w-4" />
      </Button>
    </div>
  </div>
</template>
