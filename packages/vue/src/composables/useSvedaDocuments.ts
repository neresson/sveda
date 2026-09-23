import { computed, inject, ref } from 'vue';
import { useSvedaT } from '../i18n/index';
import { SvedaClientKey } from '../plugin';

export type SvedaExtractedDocumentItem = {
  filename?: string;
  ok?: boolean;
  text?: string;
  truncated?: boolean;
  error?: string;
};

const DOCUMENT_ERROR_KEY_BY_CODE: Record<string, string> = {
  read_failed: 'chatDocumentErrorReadFailed',
  empty: 'chatDocumentErrorEmpty',
  quota_exceeded: 'chatDocumentErrorQuotaExceeded',
  unknown: 'chatDocumentErrorUnknown',
};

export function useSvedaDocuments(
  options: {
    endpoint?: string;
    onError?: (message: string) => void;
  } = {}
) {
  const t = useSvedaT();
  const client = inject(SvedaClientKey, null);

  const extractingDocuments = ref(false);
  const documentStatusBanner = computed(() =>
    extractingDocuments.value ? t('chatDocumentsExtracting') : ''
  );

  const resolveEndpoint = (): string | null =>
    options.endpoint ?? client?.endpoints.documentsExtract ?? null;

  const buildChatDocumentSections = (items: SvedaExtractedDocumentItem[]): string => {
    const rows: string[] = [];
    rows.push(t('chatDocumentsHeader'));
    for (const item of items || []) {
      rows.push('');
      rows.push(`--- ${item.filename} ---`);
      if (item.ok && item.text) {
        rows.push(item.text);
        if (item.truncated) {
          rows.push(t('chatDocumentTruncatedNotice'));
        }
      } else {
        const code = typeof item.error === 'string' ? item.error : 'unknown';
        rows.push(t(DOCUMENT_ERROR_KEY_BY_CODE[code] ?? 'chatDocumentErrorUnknown'));
      }
    }
    return rows.join('\n');
  };

  const extractChatDocuments = async (
    rawFiles: File[]
  ): Promise<{ items: SvedaExtractedDocumentItem[] }> => {
    const endpoint = resolveEndpoint();
    if (!endpoint || !client) {
      throw new Error(t('chatDocumentsExtractFailed'));
    }

    const formData = new FormData();
    for (const file of rawFiles) {
      formData.append('files[]', file);
    }

    const response = await client.fetchImpl(endpoint, {
      method: 'POST',
      headers: {
        Accept: 'application/json',
        ...client.resolveHeaders(),
      },
      credentials: client.credentials,
      body: formData,
    });

    const payload = (await response.json().catch(() => ({}))) as {
      message?: string;
      errors?: Record<string, unknown>;
      items?: SvedaExtractedDocumentItem[];
    };

    if (!response.ok) {
      let message = payload?.message || t('chatDocumentsExtractFailed');
      const errors = payload?.errors;
      if (errors && typeof errors === 'object') {
        const values = Object.values(errors).flat();
        const first = values[0];
        if (first != null && String(first).trim() !== '') {
          message = Array.isArray(first) ? String(first[0]) : String(first);
        }
      }

      throw new Error(message);
    }

    return { items: Array.isArray(payload.items) ? payload.items : [] };
  };

  const showDocumentExtractError = (error: unknown) => {
    options.onError?.(
      error instanceof Error && error.message ? error.message : t('chatDocumentsExtractFailed')
    );
  };

  return {
    extractingDocuments,
    documentStatusBanner,
    buildChatDocumentSections,
    extractChatDocuments,
    showDocumentExtractError,
  };
}
