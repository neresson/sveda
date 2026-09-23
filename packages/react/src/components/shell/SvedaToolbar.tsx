import { History, Maximize2, MessageSquare, Minimize2, PanelRight, Sparkles, X } from 'lucide-react';
import { useSvedaT } from '../../provider';
import { cn } from '../../lib/utils';
import type { SvedaChatViewMode } from '../../hooks/useSvedaChatLayout';

const iconButtonClass =
  'inline-flex h-9 w-9 items-center justify-center bg-transparent text-muted-foreground hover:text-foreground';

export interface SvedaToolbarProps {
  title: string;
  isMobile: boolean;
  viewMode: SvedaChatViewMode;
  isImmersiveDesktop: boolean;
  showHistorySidebar: boolean;
  viewModeFloatingLabel: string;
  viewModeFixedLabel: string;
  immersiveEnterLabel: string;
  immersiveExitLabel: string;
  immersiveBrandName?: string;
  immersiveBrandLogo?: string;
  onToggleHistory: () => void;
  onEnterImmersive: () => void;
  onExitImmersive: () => void;
  onToggleViewMode: () => void;
  onMinimize: () => void;
}

export function SvedaToolbar({
  title,
  isMobile,
  viewMode,
  isImmersiveDesktop,
  showHistorySidebar,
  viewModeFloatingLabel,
  viewModeFixedLabel,
  immersiveEnterLabel,
  immersiveExitLabel,
  immersiveBrandName = '',
  immersiveBrandLogo = '',
  onToggleHistory,
  onEnterImmersive,
  onExitImmersive,
  onToggleViewMode,
  onMinimize,
}: SvedaToolbarProps) {
  const t = useSvedaT();
  const headerClass = cn(
    'flex h-12 shrink-0 flex-row items-center justify-between border-b border-border bg-card px-3',
    viewMode === 'floating' && !isMobile ? 'rounded-t-[var(--sveda-radius)]' : 'rounded-none'
  );

  return (
    <header className={headerClass}>
      {!isImmersiveDesktop ? (
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <button
            type="button"
            className={cn(iconButtonClass, showHistorySidebar ? 'bg-muted/60 text-foreground' : '')}
            aria-label={t('chatHistory')}
            title={t('chatHistory')}
            onClick={onToggleHistory}
          >
            <History className="h-4 w-4" />
          </button>
          <h2 className="min-w-0 flex-1 truncate text-left font-mono text-[11px] font-normal uppercase tracking-[0.14em] text-foreground">
            {title}
          </h2>
        </div>
      ) : (
        <div className="flex min-w-0 flex-1 items-center gap-4">
          <div className="w-[min(20rem,33vw)] min-w-0">
            <div className="flex items-center gap-2">
              {immersiveBrandLogo ? (
                <img src={immersiveBrandLogo} alt={immersiveBrandName} className="size-7 shrink-0 rounded" />
              ) : (
                <Sparkles className="size-5 shrink-0 text-primary" />
              )}
              <span className="truncate text-lg font-bold leading-none text-foreground">{immersiveBrandName}</span>
            </div>
          </div>
          <h2 className="min-w-0 flex-1 truncate text-left font-mono text-[11px] font-normal uppercase tracking-[0.14em] text-foreground">
            {title}
          </h2>
        </div>
      )}
      <div className="flex shrink-0 items-center gap-0.5">
        {!isMobile && viewMode === 'immersive' ? (
          <button
            type="button"
            className={iconButtonClass}
            aria-label={immersiveExitLabel}
            title={immersiveExitLabel}
            onClick={onExitImmersive}
          >
            <Minimize2 className="h-4 w-4" />
          </button>
        ) : null}
        {!isMobile && viewMode !== 'immersive' ? (
          <button
            type="button"
            className={iconButtonClass}
            aria-label={immersiveEnterLabel}
            title={immersiveEnterLabel}
            onClick={onEnterImmersive}
          >
            <Maximize2 className="h-4 w-4" />
          </button>
        ) : null}
        {!isMobile && viewMode !== 'immersive' ? (
          <button
            type="button"
            className={iconButtonClass}
            aria-label={viewMode === 'floating' ? viewModeFixedLabel : viewModeFloatingLabel}
            title={viewMode === 'floating' ? viewModeFixedLabel : viewModeFloatingLabel}
            onClick={onToggleViewMode}
          >
            {viewMode === 'floating' ? <PanelRight className="h-4 w-4" /> : <MessageSquare className="h-4 w-4" />}
          </button>
        ) : null}
        <button
          type="button"
          className={iconButtonClass}
          aria-label={t('toolbarMinimize')}
          title={t('toolbarMinimize')}
          onClick={onMinimize}
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}
