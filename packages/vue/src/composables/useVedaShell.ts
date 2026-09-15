import { computed, type ComputedRef, type Ref } from 'vue';
import { useVedaLauncher } from '../appearance';
import { useVedaT } from '../i18n/index';

export interface VedaBrandInfo {
  name?: string;
  logoUrl?: string | null;
}

type CurrentChat =
  | {
      title?: string;
    }
  | null
  | undefined;

export function useVedaShell(
  currentChat: Ref<CurrentChat>,
  brand: ComputedRef<VedaBrandInfo | undefined>,
  layout: {
    isMobile: ComputedRef<boolean>;
    viewMode: Ref<string>;
    chatHeight: Ref<number>;
  }
) {
  const t = useVedaT();
  const launcher = useVedaLauncher();

  const headerTitle = computed(() => currentChat.value?.title || t('chatTitle'));
  const brandDisplayName = computed(() => brand.value?.name || 'Veda');
  const brandLogo = computed(() => brand.value?.logoUrl || null);
  const launcherLabel = computed(() => {
    const custom = launcher.label.trim();

    return custom !== '' ? custom : brandDisplayName.value;
  });
  const launcherIcon = computed(() => launcher.icon);
  const launcherImage = computed(() => {
    const uploaded = launcher.image.trim();
    if (uploaded !== '') {
      return uploaded;
    }

    return brandLogo.value || '';
  });

  const nonImmersiveShellClass = computed(() => {
    if (layout.isMobile.value || layout.viewMode.value === 'fixed') {
      return 'relative flex h-full min-h-0 min-w-0 flex-1 flex-row';
    }
    return 'flex min-h-0 flex-row items-stretch gap-2';
  });

  const historyAsideSurfaceClass = computed(() => {
    const base =
      'veda-chat-surface flex min-h-0 shrink-0 flex-col overflow-hidden bg-background';
    if (layout.isMobile.value) {
      return `${base} h-full w-[min(20rem,88vw)] border-r border-border`;
    }
    if (layout.viewMode.value === 'fixed') {
      return `${base} absolute right-full top-0 z-10 h-full w-[min(20rem,40vw)] border-r border-border shadow-sm`;
    }
    return `${base} h-full w-[min(20rem,40vw)] rounded-[var(--veda-radius)] border border-border shadow-sm`;
  });

  const floatNonImmersiveShellStyle = computed(() => {
    if (layout.isMobile.value || layout.viewMode.value !== 'floating') {
      return undefined;
    }
    return { height: `${layout.chatHeight.value}px`, minHeight: 0 };
  });

  return {
    headerTitle,
    brandDisplayName,
    brandLogo,
    launcherLabel,
    launcherIcon,
    launcherImage,
    nonImmersiveShellClass,
    historyAsideSurfaceClass,
    floatNonImmersiveShellStyle,
  };
}
