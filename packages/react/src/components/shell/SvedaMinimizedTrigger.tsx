import type { LucideIcon } from 'lucide-react';
import { svedaLauncherIconComponent } from '../../launcherIcons';
import { cn } from '../../lib/utils';

const triggerClass =
  'inline-flex h-auto min-w-0 items-center justify-center gap-2 rounded-none border border-primary bg-primary px-3.5 py-2 font-mono text-[11px] tracking-[0.14em] text-primary-foreground shadow-none transition-opacity hover:opacity-90 [&_svg]:!h-3.5 [&_svg]:!w-3.5';

export interface SvedaMinimizedTriggerProps {
  label: string;
  icon?: string;
  logoSrc?: string;
  logoAlt?: string;
  onOpen: () => void;
}

export function SvedaMinimizedTrigger({
  label,
  icon = 'sparkles',
  logoSrc = '',
  logoAlt = '',
  onOpen,
}: SvedaMinimizedTriggerProps) {
  const Icon: LucideIcon = svedaLauncherIconComponent(icon);

  return (
    <button type="button" className={cn(triggerClass)} onClick={onOpen} data-testid="sveda-launcher">
      {logoSrc ? (
        <img
          src={logoSrc}
          alt={logoAlt}
          className="h-6 w-6 flex-shrink-0 object-cover min-[1872px]:h-10 min-[1872px]:w-10"
        />
      ) : (
        <Icon className="h-6 w-6 flex-shrink-0" />
      )}
      {label ? (
        <span className="font-mono text-[11px] font-normal tracking-[0.14em]">{label}</span>
      ) : null}
    </button>
  );
}
