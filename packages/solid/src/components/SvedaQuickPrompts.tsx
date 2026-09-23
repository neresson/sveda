import { For, type Component } from 'solid-js';
import { cn } from '../lib/utils';

export interface SvedaQuickPromptItem {
  label: string;
  prompt: string;
}

export interface SvedaQuickPromptsProps {
  prompts: SvedaQuickPromptItem[];
  title: string;
  disabled?: boolean;
  class?: string;
  onSelect: (prompt: string) => void;
}

export const SvedaQuickPrompts: Component<SvedaQuickPromptsProps> = props => {
  return (
    <div class={cn('w-full', props.class)} role="group" aria-label={props.title}>
      <div class="quick-prompts-row -mx-1 flex gap-2 overflow-x-auto px-1 pb-1">
        <For each={props.prompts}>
          {prompt => (
            <button
              type="button"
              class="h-8 shrink-0 rounded-[var(--sveda-radius)] border border-border/60 bg-muted/80 px-3 text-xs font-normal text-muted-foreground shadow-none hover:bg-muted hover:text-foreground disabled:opacity-50"
              disabled={props.disabled}
              onClick={() => props.onSelect(prompt.prompt)}
            >
              {prompt.label}
            </button>
          )}
        </For>
      </div>
    </div>
  );
};
