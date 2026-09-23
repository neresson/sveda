import { History, Maximize2, MessageSquare, Minimize2, PanelRight, Sparkles, X } from 'lucide-solid';
import { type Component, Show, createMemo } from 'solid-js';
import { useSvedaT } from '../i18n/index';
import { cn } from '../lib/utils';

const iconButtonClass =
  'inline-flex h-9 w-9 items-center justify-center bg-transparent text-muted-foreground hover:text-foreground';

export interface SvedaToolbarProps {
  isMobile: boolean;
  viewMode: string;
  isImmersiveDesktop?: boolean;
  showHistorySidebar?: boolean;
  title: string;
  viewModeFloatingLabel?: string;
  viewModeFixedLabel?: string;
  immersiveEnterLabel?: string;
  immersiveExitLabel?: string;
  immersiveBrandName?: string;
  immersiveBrandLogo?: string;
  onToggleHistorySidebar?: () => void;
  onToggleViewMode?: () => void;
  onEnterImmersive?: () => void;
  onExitImmersive?: () => void;
  onMinimize: () => void;
}

export const SvedaToolbar: Component<SvedaToolbarProps> = props => {
  const t = useSvedaT();

  const headerClass = createMemo(() => {
    const base =
      'flex h-12 shrink-0 flex-row items-center justify-between border-b border-border bg-card px-3';
    if (props.viewMode === 'floating' && !props.isMobile) {
      return `${base} rounded-t-[var(--sveda-radius)]`;
    }
    return `${base} rounded-none`;
  });

  return (
    <header class={headerClass()}>
      <Show
        when={!props.isImmersiveDesktop}
        fallback={
          <div class="flex min-w-0 flex-1 items-center gap-4">
            <div class="w-[min(20rem,33vw)] min-w-0">
              <div class="flex items-center gap-2">
                <Show
                  when={props.immersiveBrandLogo}
                  fallback={<Sparkles class="size-5 shrink-0 text-primary" />}
                >
                  <img
                    src={props.immersiveBrandLogo}
                    alt={props.immersiveBrandName ?? ''}
                    class="size-7 shrink-0 rounded"
                  />
                </Show>
                <span class="truncate text-lg font-bold leading-none text-foreground">
                  {props.immersiveBrandName}
                </span>
              </div>
            </div>
            <h2 class="min-w-0 flex-1 truncate text-left font-mono text-[11px] font-normal uppercase tracking-[0.14em] text-foreground">
              {props.title}
            </h2>
          </div>
        }
      >
        <div class="flex min-w-0 flex-1 items-center gap-2">
          <button
            type="button"
            class={cn(iconButtonClass, props.showHistorySidebar ? 'bg-muted/60 text-foreground' : '')}
            aria-label={t('chatHistory')}
            title={t('chatHistory')}
            onClick={() => props.onToggleHistorySidebar?.()}
          >
            <History class="h-4 w-4" />
          </button>
          <h2 class="min-w-0 flex-1 truncate text-left font-mono text-[11px] font-normal uppercase tracking-[0.14em] text-foreground">
            {props.title}
          </h2>
        </div>
      </Show>
      <div class="flex shrink-0 items-center gap-0.5">
        <Show when={!props.isMobile && props.viewMode === 'immersive'}>
          <button
            type="button"
            class={iconButtonClass}
            aria-label={props.immersiveExitLabel ?? t('immersiveModeExit')}
            title={props.immersiveExitLabel ?? t('immersiveModeExit')}
            onClick={() => props.onExitImmersive?.()}
          >
            <Minimize2 class="h-4 w-4" />
          </button>
        </Show>
        <Show when={!props.isMobile && props.viewMode !== 'immersive'}>
          <button
            type="button"
            class={iconButtonClass}
            aria-label={props.immersiveEnterLabel ?? t('immersiveModeEnter')}
            title={props.immersiveEnterLabel ?? t('immersiveModeEnter')}
            onClick={() => props.onEnterImmersive?.()}
          >
            <Maximize2 class="h-4 w-4" />
          </button>
        </Show>
        <Show when={!props.isMobile && props.viewMode !== 'immersive'}>
          <button
            type="button"
            class={iconButtonClass}
            aria-label={
              props.viewMode === 'floating'
                ? (props.viewModeFixedLabel ?? t('viewModeFixed'))
                : (props.viewModeFloatingLabel ?? t('viewModeFloating'))
            }
            title={
              props.viewMode === 'floating'
                ? (props.viewModeFixedLabel ?? t('viewModeFixed'))
                : (props.viewModeFloatingLabel ?? t('viewModeFloating'))
            }
            onClick={() => props.onToggleViewMode?.()}
          >
            <Show when={props.viewMode === 'floating'} fallback={<MessageSquare class="h-4 w-4" />}>
              <PanelRight class="h-4 w-4" />
            </Show>
          </button>
        </Show>
        <button
          type="button"
          class={iconButtonClass}
          aria-label={t('toolbarMinimize')}
          title={t('toolbarMinimize')}
          onClick={() => props.onMinimize()}
        >
          <X class="h-4 w-4" />
        </button>
      </div>
    </header>
  );
};
