import { type Component, Show } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import { svedaLauncherIconComponent } from '../launcherIcons';
import { cn } from '../lib/utils';

export interface SvedaMinimizedTriggerProps {
  label: string;
  icon?: string;
  logoSrc?: string;
  logoAlt?: string;
  onOpen: () => void;
}

const triggerClass =
  'inline-flex h-auto min-w-0 items-center justify-center gap-2 rounded-none border border-primary bg-primary px-3.5 py-2 font-mono text-[11px] tracking-[0.14em] text-primary-foreground shadow-none transition-opacity hover:opacity-90 [&_svg]:!h-3.5 [&_svg]:!w-3.5';

export const SvedaMinimizedTrigger: Component<SvedaMinimizedTriggerProps> = props => {
  return (
    <button type="button" class={cn(triggerClass)} onClick={() => props.onOpen()}>
      <Show
        when={props.logoSrc}
        fallback={
          <Dynamic
            component={svedaLauncherIconComponent(props.icon ?? 'sparkles')}
            class="h-6 w-6 flex-shrink-0"
          />
        }
      >
        <img
          src={props.logoSrc}
          alt={props.logoAlt ?? ''}
          class="h-6 w-6 flex-shrink-0 object-cover min-[1872px]:h-10 min-[1872px]:w-10"
        />
      </Show>
      <Show when={props.label}>
        <span class="font-mono text-[11px] font-normal tracking-[0.14em]">{props.label}</span>
      </Show>
    </button>
  );
};
