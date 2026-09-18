import { computed, type ComputedRef, type Ref } from 'vue';
import { useSvedaLauncher } from '../appearance';
import { useSvedaT } from '../i18n/index';

export interface SvedaBrandInfo {
  name?: string;
  logoUrl?: string | null;
}

type CurrentChat =
  | {
      title?: string;
    }
  | null
  | undefined;

export function useSvedaShell(
  currentChat: Ref<CurrentChat>,
  brand: ComputedRef<SvedaBrandInfo | undefined>,
  layout: {
    isMobile: ComputedRef<boolean>;
    viewMode: Ref<string>;
    chatHeight: Ref<number>;
    fillHost?: boolean;
  }
) {
  const t = useSvedaT();
  const launcher = useSvedaLauncher();

  const headerTitle = computed(() => currentChat.value?.title || t('chatTitle'));
  const brandDisplayName = computed(() => brand.value?.name || 'Sveda');
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
    if (layout.fillHost) {
      return 'relative flex h-full min-h-0 min-w-0 w-full flex-row items-stretch overflow-hidden';
    }
    return 'relative flex min-h-0 flex-row items-stretch gap-2';
  });

  const historyAsideSurfaceClass = computed(() => {
    const base =
      'sveda-chat-surface flex min-h-0 w-80 shrink-0 flex-col overflow-hidden bg-background';
    if (layout.isMobile.value) {
      return `${base} h-full max-w-[88vw] border-r border-border`;
    }
    if (layout.fillHost) {
      return `${base} h-full border-r border-border`;
    }
    if (layout.viewMode.value === 'fixed') {
      return `${base} absolute right-full top-0 z-10 h-full border-r border-border`;
    }
    return `${base} h-full rounded-[var(--sveda-radius)] border border-border`;
  });

  const floatNonImmersiveShellStyle = computed(() => {
    if (layout.fillHost) {
      return { height: '100%', minHeight: 0 };
    }
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
