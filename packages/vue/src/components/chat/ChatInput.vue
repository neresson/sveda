<script setup>
  import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
  } from '../../ui/alert-dialog/index.js';
  import { Badge } from '../../ui/badge/index.js';
  import { Button } from '../../ui/button/index.js';
  import { Textarea } from '../../ui/textarea/index.js';
  import { useToast } from '../../ui/toast/index.js';
  import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/tooltip/index.js';
  import { VedaI18nKey, useVedaT } from '../../i18n/index.js';
  import { Loader2, Mic, Paperclip, Send, Square, X } from 'lucide-vue-next';
  import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';

  const t = useVedaT();
  const vedaI18n = inject(VedaI18nKey, null);
  const locale = computed(() => vedaI18n?.locale ?? '');
  const { toast } = useToast();

  const inputMessage = defineModel('modelValue', {
    type: String,
    default: '',
  });

  const pendingFiles = defineModel('pendingFiles', {
    type: Array,
    default: () => [],
  });

  const props = defineProps({
    isLoading: {
      type: Boolean,
      default: false,
    },
    hasCurrentChat: {
      type: Boolean,
      default: false,
    },
    isStreaming: {
      type: Boolean,
      default: false,
    },
    noBorder: {
      type: Boolean,
      default: false,
    },
    statusBanner: {
      type: String,
      default: '',
    },
    acceptFiles: {
      type: String,
      default: '.pdf,.doc,.docx,.xls,.xlsx,.xlsm,.ods,.ppt,.pptx,.pps,.ppsx,.txt,.csv,.jpg,.jpeg,.png,.gif,.webp,.bmp,.tif,.tiff',
    },
    maxFiles: {
      type: Number,
      default: 5,
    },
  });

  const emit = defineEmits(['send', 'stop']);

  const textareaRef = ref(null);
  const fileInputRef = ref(null);
  const isDragActive = ref(false);
  let dragDepth = 0;
  const MIN_HEIGHT = 60;
  const MAX_HEIGHT = 250;

  const speechSupported = computed(() => {
    if (typeof window === 'undefined') return false;
    return Boolean(window.SpeechRecognition || window.webkitSpeechRecognition);
  });

  const isVoiceInputPreferredBrowser = computed(() => {
    if (typeof navigator === 'undefined') return true;
    const ua = navigator.userAgent || '';
    if (/Edg\//.test(ua) || /EdgiOS/.test(ua)) return true;
    if (/CriOS/.test(ua)) return true;
    if (/Chrome\//.test(ua) && !/Edg\//.test(ua) && !/OPR\//.test(ua) && !/SamsungBrowser/i.test(ua)) {
      return true;
    }
    if (/Safari\//.test(ua) && !/Chrome\//.test(ua) && !/CriOS/.test(ua) && !/Android/.test(ua)) {
      return true;
    }
    return false;
  });

  const speechLang = computed(() => {
    const l = String(locale.value || '');
    if (l.startsWith('ru')) return 'ru-RU';
    if (l.startsWith('en')) return 'en-US';
    if (typeof navigator !== 'undefined' && navigator.language) return navigator.language;
    return 'en-US';
  });

  const recognitionRef = ref(null);
  const isListening = ref(false);
  const browserWarningOpen = ref(false);
  const voiceSessionBase = ref('');

  const joinSpeechParts = (left, right) => {
    if (!right) return left || '';
    if (!left) return right;
    if (/\s$/.test(left) || /^\s/.test(right)) return left + right;
    return `${left} ${right}`;
  };

  const applyVoiceResults = event => {
    let fullFinal = '';
    let interim = '';
    for (let i = 0; i < event.results.length; i++) {
      const res = event.results[i];
      const text = res[0].transcript;
      if (res.isFinal) {
        fullFinal = joinSpeechParts(fullFinal, text);
      } else {
        interim = joinSpeechParts(interim, text);
      }
    }
    inputMessage.value = joinSpeechParts(joinSpeechParts(voiceSessionBase.value, fullFinal), interim);
  };

  const resetVoiceSessionState = () => {
    voiceSessionBase.value = '';
  };

  const ensureRecognition = () => {
    const Ctor = typeof window !== 'undefined' && (window.SpeechRecognition || window.webkitSpeechRecognition);
    if (!Ctor) return null;
    if (recognitionRef.value) return recognitionRef.value;
    const recognition = new Ctor();
    recognition.continuous = true;
    recognition.interimResults = true;

    recognition.onresult = event => {
      applyVoiceResults(event);
    };

    recognition.onerror = event => {
      if (event.error === 'aborted' || event.error === 'no-speech') return;
      isListening.value = false;
      if (event.error === 'not-allowed') {
        toast({
          title: t('voiceInputError'),
          description: t('voiceInputPermissionDenied'),
          variant: 'destructive',
        });
      } else {
        toast({
          title: t('voiceInputError'),
          description: t('voiceInputErrorGeneric'),
          variant: 'destructive',
        });
      }
    };

    recognition.onend = () => {
      if (isListening.value) {
        try {
          recognition.start();
        } catch {
          isListening.value = false;
          resetVoiceSessionState();
        }
      } else {
        resetVoiceSessionState();
      }
    };

    recognitionRef.value = recognition;
    return recognition;
  };

  const stopVoiceInput = () => {
    isListening.value = false;
    const r = recognitionRef.value;
    if (r) {
      try {
        r.stop();
      } catch {
        try {
          r.abort();
        } catch {}
      }
    }
  };

  const startVoiceInput = () => {
    const recognition = ensureRecognition();
    if (!recognition) {
      toast({
        title: t('voiceInputError'),
        description: t('voiceInputUnsupported'),
        variant: 'destructive',
      });
      return;
    }
    recognition.lang = speechLang.value;
    voiceSessionBase.value = inputMessage.value;
    try {
      recognition.start();
      isListening.value = true;
    } catch {
      isListening.value = false;
      resetVoiceSessionState();
      toast({
        title: t('voiceInputError'),
        description: t('voiceInputErrorGeneric'),
        variant: 'destructive',
      });
    }
  };

  const confirmBrowserWarningAndStart = () => {
    browserWarningOpen.value = false;
    startVoiceInput();
  };

  const onVoiceMicClick = () => {
    if (!isVoiceInputPreferredBrowser.value) {
      browserWarningOpen.value = true;
    } else {
      startVoiceInput();
    }
  };

  const voiceInputDisabled = computed(() => props.isLoading || !props.hasCurrentChat || props.isStreaming);

  const attachmentToolbarDisabled = computed(() => props.isLoading || !props.hasCurrentChat || props.isStreaming || isListening.value);

  const normalizePendingList = raw => {
    if (!Array.isArray(raw)) {
      return [];
    }
    return raw.filter(entry => entry instanceof File);
  };

  const mergeFileList = incoming => {
    const base = normalizePendingList(pendingFiles.value);
    const merged = [...base];
    const seen = new Set(base.map(f => `${f.name}:${f.size}:${f.lastModified}`));
    for (const file of incoming) {
      if (!(file instanceof File)) continue;
      const key = `${file.name}:${file.size}:${file.lastModified}`;
      if (seen.has(key)) continue;
      seen.add(key);
      merged.push(file);
    }
    return merged.slice(0, props.maxFiles);
  };

  const queueFilesFromList = fileList => {
    const next = mergeFileList(Array.from(fileList || []));
    pendingFiles.value = next;
  };

  const openFilePicker = () => {
    if (attachmentToolbarDisabled.value) return;
    fileInputRef.value?.click?.();
  };

  const removePendingAt = index => {
    const list = normalizePendingList(pendingFiles.value);
    list.splice(index, 1);
    pendingFiles.value = [...list];
  };

  const onFileInputChange = event => {
    const inputEl = event?.target;
    queueFilesFromList(inputEl?.files);
    if (inputEl) {
      inputEl.value = '';
    }
  };

  const onDragEnter = e => {
    if (attachmentToolbarDisabled.value) return;
    e.preventDefault();
    dragDepth += 1;
    isDragActive.value = true;
  };

  const onDragLeave = e => {
    if (attachmentToolbarDisabled.value) return;
    e.preventDefault();
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) {
      isDragActive.value = false;
    }
  };

  const onDragOver = e => {
    if (attachmentToolbarDisabled.value) return;
    e.preventDefault();
  };

  const onDrop = e => {
    if (attachmentToolbarDisabled.value) return;
    e.preventDefault();
    dragDepth = 0;
    isDragActive.value = false;
    queueFilesFromList(e.dataTransfer?.files);
  };

  const onPaste = e => {
    if (attachmentToolbarDisabled.value || isListening.value) return;
    const dt = e.clipboardData;
    const files = dt?.files?.length ? Array.from(dt.files) : [];
    if (files.length === 0) return;
    e.preventDefault();
    queueFilesFromList(files);
  };

  const canSend = computed(() => {
    const hasText = Boolean(String(inputMessage.value || '').trim());
    const hasFiles = normalizePendingList(pendingFiles.value).length > 0;
    return (hasText || hasFiles) && props.hasCurrentChat;
  });

  const hasStatusBanner = computed(() => String(props.statusBanner || '').trim() !== '');
  const controlIconClass = 'h-4 w-4';

  const micIdleButtonClass = 'h-8 w-8 rounded-[var(--veda-radius)] text-muted-foreground hover:text-foreground';

  const micStopButtonClass = 'h-8 w-8 rounded-[var(--veda-radius)] text-destructive hover:bg-destructive/10 hover:text-destructive';

  const sendButtonClass =
    'h-8 w-8 rounded-[var(--veda-radius)] text-brand-primary-purple hover:bg-brand-primary-purple/10 hover:text-brand-primary-purple disabled:opacity-50 dark:text-purple-400 dark:hover:bg-purple-400/10 dark:hover:text-purple-300';

  const streamStopButtonClass = 'h-8 w-8 rounded-[var(--veda-radius)] text-destructive hover:bg-destructive/10 hover:text-destructive';

  const controlToolbarClass = 'flex items-center justify-between px-2';

  const adjustTextareaHeight = () => {
    nextTick(() => {
      if (textareaRef.value) {
        const textarea = textareaRef.value.$el;
        if (textarea && textarea.tagName === 'TEXTAREA') {
          textarea.style.height = 'auto';
          const scrollHeight = textarea.scrollHeight;
          const newHeight = Math.min(Math.max(scrollHeight, MIN_HEIGHT), MAX_HEIGHT);
          textarea.style.height = `${newHeight}px`;
          textarea.style.overflowY = scrollHeight > MAX_HEIGHT ? 'auto' : 'hidden';
        }
      }
    });
  };

  watch(
    () => inputMessage.value,
    () => {
      adjustTextareaHeight();
    }
  );

  onMounted(() => {
    adjustTextareaHeight();
  });

  onBeforeUnmount(() => {
    isListening.value = false;
    resetVoiceSessionState();
    const r = recognitionRef.value;
    if (r) {
      try {
        r.abort();
      } catch {}
      recognitionRef.value = null;
    }
  });

  const handleKeyPress = event => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      emit('send');
    }
  };

  const handleInput = () => {
    adjustTextareaHeight();
  };
</script>

<template>
  <div
    :class="
      noBorder
        ? 'flex flex-shrink-0 flex-col bg-transparent p-4'
        : 'veda-chat-surface flex flex-shrink-0 flex-col border-t border-border/50 p-4 dark:border-slate-700'
    "
  >
    <div
      v-if="hasStatusBanner"
      class="mb-3 flex items-center gap-2 rounded-lg border border-border/50 bg-muted/50 px-3 py-2 text-sm text-muted-foreground"
    >
      <Loader2 class="h-4 w-4 shrink-0 animate-spin" />
      <span class="leading-snug">{{ statusBanner }}</span>
    </div>
    <input
      ref="fileInputRef"
      type="file"
      class="hidden"
      :accept="acceptFiles"
      multiple
      @change="onFileInputChange"
    />
    <div
      class="border-input-transparent relative flex flex-col rounded-[var(--veda-radius)] border bg-background shadow-sm transition-colors"
      :class="isDragActive && !attachmentToolbarDisabled ? 'border-brand-primary-purple/70 ring-1 ring-brand-primary-purple/30' : ''"
      @dragenter="onDragEnter"
      @dragleave="onDragLeave"
      @dragover="onDragOver"
      @drop="onDrop"
    >
      <div
        v-if="normalizePendingList(pendingFiles).length > 0"
        class="flex flex-wrap gap-2 border-b border-border/40 px-3 py-2"
      >
        <Badge
          v-for="(file, idx) in normalizePendingList(pendingFiles)"
          :key="`${file.name}-${file.size}-${idx}`"
          variant="secondary"
          class="max-w-[min(100%,14rem)] gap-1 pr-1"
        >
          <span class="truncate">{{ file.name }}</span>
          <button
            type="button"
            class="rounded-[var(--veda-radius)] p-0.5 hover:bg-muted"
            :disabled="attachmentToolbarDisabled"
            :aria-label="t('chatRemoveFile')"
            @click="removePendingAt(idx)"
          >
            <X class="h-3 w-3" />
          </button>
        </Badge>
      </div>
      <Textarea
        ref="textareaRef"
        v-model="inputMessage"
        @input="handleInput"
        @keypress="handleKeyPress"
        @paste="onPaste"
        :placeholder="t('typeMessage')"
        :disabled="isLoading"
        :readonly="isListening"
        class="veda-chat-input-textarea max-h-[250px] min-h-[60px] w-full resize-none border-0 bg-transparent p-3 shadow-none !outline-none !ring-0 focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0"
        style="height: 60px; outline: none; box-shadow: none"
      />

      <div :class="controlToolbarClass">
        <div class="flex min-w-0 flex-1 items-center gap-1">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  :class="micIdleButtonClass"
                  :disabled="attachmentToolbarDisabled"
                  @click="openFilePicker"
                >
                  <Paperclip :class="controlIconClass" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>{{ t('chatAttachFiles') }}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <div class="min-w-0 flex-1">
            <slot name="bottom-left"></slot>
          </div>
        </div>

        <div class="flex items-center gap-1">
          <TooltipProvider v-if="speechSupported && !isStreaming">
            <Tooltip v-if="!isListening">
              <TooltipTrigger as-child>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  :class="micIdleButtonClass"
                  :disabled="voiceInputDisabled"
                  @click="onVoiceMicClick"
                >
                  <Mic :class="controlIconClass" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>{{ t('voiceInputStart') }}</p>
              </TooltipContent>
            </Tooltip>
            <Tooltip v-else>
              <TooltipTrigger as-child>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  :class="micStopButtonClass"
                  @click="stopVoiceInput"
                >
                  <Square :class="controlIconClass" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>{{ t('voiceInputStop') }}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>

          <Button
            v-if="isStreaming"
            variant="ghost"
            @click="$emit('stop')"
            size="icon"
            :class="streamStopButtonClass"
          >
            <Square :class="controlIconClass" />
          </Button>
          <Button
            v-else
            variant="ghost"
            @click="$emit('send')"
            :disabled="isLoading || !canSend"
            size="icon"
            :class="sendButtonClass"
          >
            <Loader2
              v-if="isLoading"
              :class="[controlIconClass, 'animate-spin']"
            />
            <Send
              v-else
              :class="controlIconClass"
            />
          </Button>
        </div>
      </div>
    </div>

    <div class="mt-2">
      <slot name="bottom-outside"></slot>
    </div>

    <AlertDialog v-model:open="browserWarningOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{{ t('voiceInputBrowserWarningTitle') }}</AlertDialogTitle>
          <AlertDialogDescription>
            {{ t('voiceInputBrowserWarningDescription') }}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>{{ t('cancel') }}</AlertDialogCancel>
          <AlertDialogAction @click="confirmBrowserWarningAndStart">
            {{ t('voiceInputBrowserWarningContinue') }}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
