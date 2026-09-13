<script setup>
  import { Button } from '../../ui/button/index.js';
  import { Input } from '../../ui/input/index.js';
  import { ScrollArea } from '../../ui/scroll-area/index.js';
  import { useVedaT } from '../../i18n/index.js';
  import { Search, SquarePen, Trash2, X } from 'lucide-vue-next';
  import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

  const t = useVedaT();

  const props = defineProps({
    show: {
      type: Boolean,
      required: true,
    },
    histories: {
      type: Array,
      required: true,
    },
    currentChatId: {
      type: String,
      default: null,
    },
    height: {
      type: [Number, String],
      default: 600,
    },
    variant: {
      type: String,
      default: 'dropdown',
      validator: v => v === 'dropdown' || v === 'sidebar',
    },
  });

  const emit = defineEmits(['close', 'select-chat', 'delete-chat', 'rename-chat']);
  const searchQuery = ref('');
  const dropdownRef = ref(null);

  const handleDeleteChat = (chatId, e) => {
    e.stopPropagation();
    emit('delete-chat', chatId);
  };

  const handleRenameChat = (chatId, e) => {
    e.stopPropagation();
    emit('rename-chat', chatId);
  };

  const getFirstUserMessage = chat => chat.messages?.find(m => m.role === 'user');

  const getChatPreview = chat => {
    if (typeof chat.preview === 'string' && chat.preview.trim()) {
      return chat.preview.trim();
    }
    return previewForUserMessage(getFirstUserMessage(chat));
  };

  const previewForUserMessage = message => {
    if (!message) return '';
    const text = typeof message.content === 'string' ? message.content.trim() : '';
    const names = Array.isArray(message.attachmentNames) ? message.attachmentNames.map(n => String(n).trim()).filter(Boolean) : [];
    const namesSnippet = names.slice(0, 3).join(', ') + (names.length > 3 ? ' …' : '');
    const attachLabel = names.length > 0 ? t('attachmentOnlyPlaceholder', { names: namesSnippet }) : '';
    if (text && attachLabel) {
      return `${text} · ${attachLabel}`;
    }
    if (text) return text;
    return attachLabel;
  };

  const normalizeText = value => String(value || '').toLowerCase();

  const filteredHistories = computed(() => {
    const query = normalizeText(searchQuery.value).trim();
    if (!query) return props.histories;

    return props.histories.filter(chat => {
      const preview = getChatPreview(chat);
      const firstUser = getFirstUserMessage(chat);
      const attachBlob = Array.isArray(firstUser?.attachmentNames) ? firstUser.attachmentNames.join(' ') : '';
      return normalizeText(chat.title).includes(query) || normalizeText(preview).includes(query) || normalizeText(attachBlob).includes(query);
    });
  });

  const getDayDiff = timestamp => {
    const targetDate = new Date(Number(timestamp) || Date.now());
    const now = new Date();
    const targetStart = new Date(targetDate.getFullYear(), targetDate.getMonth(), targetDate.getDate()).getTime();
    const nowStart = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    return Math.max(0, Math.floor((nowStart - targetStart) / 86400000));
  };

  const getDayLabel = dayDiff => {
    if (dayDiff === 0) return t('today');
    if (dayDiff === 1) return t('yesterday');
    if (dayDiff === 2) return t('dayBeforeYesterday');
    return t('daysAgo', { count: dayDiff });
  };

  const groupedHistories = computed(() => {
    const groups = [];
    const indexByLabel = new Map();

    filteredHistories.value.forEach(chat => {
      const label = getDayLabel(getDayDiff(chat.updatedAt));
      if (!indexByLabel.has(label)) {
        indexByLabel.set(label, groups.length);
        groups.push({ label, chats: [chat] });
        return;
      }
      groups[indexByLabel.get(label)].chats.push(chat);
    });

    return groups;
  });

  const onDocumentClick = event => {
    if (props.variant === 'sidebar') return;
    if (!props.show) return;
    if (!dropdownRef.value) return;
    if (!dropdownRef.value.contains(event.target)) {
      emit('close');
    }
  };

  onMounted(() => {
    document.addEventListener('mousedown', onDocumentClick);
  });

  onBeforeUnmount(() => {
    document.removeEventListener('mousedown', onDocumentClick);
  });
</script>

<template>
  <div
    v-if="variant === 'sidebar' || show"
    ref="dropdownRef"
    :class="
      variant === 'sidebar'
        ? 'flex h-full min-h-0 w-full flex-col bg-background'
        : 'absolute left-0 top-full z-50 mt-2 w-[340px] max-w-[calc(100vw-2rem)] border bg-background shadow-2xl'
    "
  >
    <div
      class="border-b p-3"
      :class="variant === 'sidebar' ? 'shrink-0' : ''"
    >
      <div class="mb-3 flex items-center justify-between">
        <p class="text-sm font-semibold">{{ t('chatHistory') }}</p>
        <Button
          v-if="variant !== 'sidebar'"
          variant="ghost"
          size="icon"
          class="h-7 w-7"
          @click="$emit('close')"
        >
          <X class="h-4 w-4" />
        </Button>
      </div>
      <div class="relative">
        <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          v-model="searchQuery"
          :placeholder="t('searchChats')"
          class="pl-9"
        />
      </div>
    </div>
    <ScrollArea :class="variant === 'sidebar' ? 'min-h-0 flex-1' : 'h-[420px]'">
      <div class="p-2">
        <div
          v-if="groupedHistories.length === 0"
          class="px-2 py-6 text-center text-sm text-muted-foreground"
        >
          {{ t('noChatsFound') }}
        </div>
        <div
          v-for="group in groupedHistories"
          :key="group.label"
          class="mb-3 last:mb-1"
        >
          <p class="px-2 pb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {{ group.label }}
          </p>
          <div
            v-for="chat in group.chats"
            :key="chat.id"
            class="group relative cursor-pointer px-2 py-2 transition-colors hover:bg-muted dark:hover:bg-slate-800"
            :class="{ 'bg-primary/10 dark:bg-primary/20': currentChatId === chat.id }"
            @click="$emit('select-chat', chat.id)"
          >
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium">{{ chat.title }}</p>
                <p class="truncate text-xs text-muted-foreground">
                  {{ getChatPreview(chat) }}
                </p>
              </div>
              <div class="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                <Button
                  variant="ghost"
                  size="icon"
                  class="h-6 w-6"
                  :aria-label="t('renameChat')"
                  @click.stop="handleRenameChat(chat.id, $event)"
                >
                  <SquarePen class="h-3 w-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  class="h-6 w-6"
                  :aria-label="t('delete')"
                  @click.stop="handleDeleteChat(chat.id, $event)"
                >
                  <Trash2 class="h-3 w-3" />
                </Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  </div>
</template>
