import { inject, ref, type ComputedRef, type Ref } from 'vue';
import { SvedaBeforeSendKey } from '../plugin';
import type { SvedaExtractedDocumentItem } from './useSvedaDocuments';

type SendMessageStreaming = (
  payload: { displayText: string; promptText: string; attachmentNames?: string[] },
  pageContext: Record<string, unknown>
) => Promise<void>;

export function useSvedaMessaging(deps: {
  isLoading: Ref<boolean>;
  currentChat: Ref<{ id?: string } | null | undefined>;
  pageContext: ComputedRef<Record<string, unknown>>;
  sendMessageStreaming: SendMessageStreaming;
  maximizeChat: () => void;
  setLoading: (value: boolean) => void;
  resetAgentCompletedNotice: () => void;
  resetMaxStepsContinueNotice?: (chatId?: string) => void;
  extractingDocuments: Ref<boolean>;
  extractChatDocuments: (files: File[]) => Promise<{ items: SvedaExtractedDocumentItem[] }>;
  buildChatDocumentSections: (items: SvedaExtractedDocumentItem[]) => string;
  onDocumentExtractError: (error: unknown) => void;
  clearAgentTasks?: () => void;
}) {
  const inputMessage = ref('');
  const pendingChatFiles = ref<File[]>([]);
  const beforeSend = inject(SvedaBeforeSendKey, null);

  const sendMessage = async () => {
    if (deps.isLoading.value || !deps.currentChat.value) {
      return;
    }
    const text = inputMessage.value.trim();
    const files = [...(pendingChatFiles.value || [])].filter(file => file instanceof File);
    if (!text && files.length === 0) {
      return;
    }

    try {
      deps.setLoading(true);
      await beforeSend?.();
    } catch {
      return;
    } finally {
      deps.setLoading(false);
    }

    deps.resetAgentCompletedNotice();
    deps.resetMaxStepsContinueNotice?.(deps.currentChat.value?.id);
    deps.clearAgentTasks?.();
    deps.maximizeChat();

    let docSection = '';
    if (files.length > 0) {
      deps.extractingDocuments.value = true;
      deps.setLoading(true);
      try {
        const extracted = await deps.extractChatDocuments(files);
        docSection = deps.buildChatDocumentSections(extracted.items);
      } catch (error) {
        deps.onDocumentExtractError(error);
        deps.setLoading(false);
        deps.extractingDocuments.value = false;
        return;
      } finally {
        deps.extractingDocuments.value = false;
        deps.setLoading(false);
      }
    }

    inputMessage.value = '';
    pendingChatFiles.value = [];

    const names = files.map(file => file.name).filter(Boolean);
    const promptPieces: string[] = [];
    if (text) {
      promptPieces.push(text);
    }
    if (docSection !== '') {
      promptPieces.push(docSection);
    }
    const promptText = promptPieces.join('\n\n');

    await deps.sendMessageStreaming(
      {
        displayText: text,
        promptText,
        attachmentNames: names.length > 0 ? names : undefined,
      },
      deps.pageContext.value
    );
  };

  const sendQuickPrompt = async (prompt: string) => {
    if (!prompt || deps.isLoading.value || !deps.currentChat.value) {
      return;
    }

    pendingChatFiles.value = [];
    inputMessage.value = prompt;
    await sendMessage();
  };

  return {
    inputMessage,
    pendingChatFiles,
    sendMessage,
    sendQuickPrompt,
  };
}
