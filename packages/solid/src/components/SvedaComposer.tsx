import { Loader2, Send, Square } from 'lucide-solid';
import { type Component, type JSX, Show, createMemo } from 'solid-js';
import { useSvedaT } from '../i18n/index';
import { cn } from '../lib/utils';

export interface SvedaComposerProps {
  value: string;
  onInput: (value: string) => void;
  isLoading?: boolean;
  confirmationPending?: boolean;
  hasCurrentChat?: boolean;
  isStreaming?: boolean;
  statusBanner?: string;
  noBorder?: boolean;
  onSend: () => void;
  onStop: () => void;
  bottomLeft?: JSX.Element;
  bottomOutside?: JSX.Element;
}

const controlIconClass = 'h-4 w-4';
const sendButtonClass =
  'inline-flex h-8 w-8 items-center justify-center rounded-none border border-dashed border-primary bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50';
const streamStopButtonClass =
  'inline-flex h-8 w-8 items-center justify-center rounded-none border border-dashed border-destructive text-destructive hover:bg-destructive/10';
const controlToolbarClass =
  'flex items-center justify-between border-t border-dashed border-foreground px-2 py-1';

export const SvedaComposer: Component<SvedaComposerProps> = props => {
  const t = useSvedaT();

  const composerDisabled = createMemo(
    () => Boolean(props.isLoading || props.confirmationPending || props.isStreaming),
  );
  const canSend = createMemo(
    () => props.value.trim().length > 0 && Boolean(props.hasCurrentChat),
  );
  const hasStatusBanner = createMemo(() => String(props.statusBanner || '').trim() !== '');

  const handleKeyPress = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      if (!composerDisabled() && canSend()) {
        props.onSend();
      }
    }
  };

  return (
    <div
      class={
        props.noBorder
          ? 'flex flex-shrink-0 flex-col bg-transparent p-4'
          : 'flex flex-shrink-0 flex-col bg-background p-3'
      }
    >
      <Show when={hasStatusBanner()}>
        <div class="mb-3 flex items-center gap-2 border border-dashed border-foreground bg-muted px-3 py-2 font-mono text-[11px] tracking-[0.08em] text-muted-foreground">
          <Loader2 class="h-4 w-4 shrink-0 animate-spin" />
          <span class="leading-snug">{props.statusBanner}</span>
        </div>
      </Show>

      <div class="sveda-chat-frame relative flex flex-col bg-background transition-colors">
        <textarea
          value={props.value}
          onInput={event => props.onInput(event.currentTarget.value)}
          onKeyPress={handleKeyPress}
          placeholder={t('typeMessage')}
          disabled={composerDisabled()}
          class="sveda-chat-input-textarea max-h-[250px] min-h-[60px] w-full resize-none border-0 bg-transparent p-3 font-sans text-sm text-foreground shadow-none !outline-none !ring-0 placeholder:font-mono placeholder:text-[11px] placeholder:tracking-[0.08em] placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0"
          style={{ height: '60px', outline: 'none', 'box-shadow': 'none' }}
        />

        <div class={controlToolbarClass}>
          <div class="flex min-w-0 flex-1 items-center gap-1">
            <div class="min-w-0 flex-1">{props.bottomLeft}</div>
          </div>

          <div class="flex items-center gap-1">
            <Show
              when={props.isStreaming}
              fallback={
                <button
                  type="button"
                  class={sendButtonClass}
                  disabled={composerDisabled() || !canSend()}
                  aria-label={t('sendMessage') || 'Send'}
                  onClick={() => props.onSend()}
                >
                  <Show
                    when={props.isLoading}
                    fallback={<Send class={controlIconClass} />}
                  >
                    <Loader2 class={cn(controlIconClass, 'animate-spin')} />
                  </Show>
                </button>
              }
            >
              <button
                type="button"
                class={streamStopButtonClass}
                aria-label={t('stopGenerating') || 'Stop'}
                onClick={() => props.onStop()}
              >
                <Square class={controlIconClass} />
              </button>
            </Show>
          </div>
        </div>
      </div>

      <div class="mt-2">{props.bottomOutside}</div>
    </div>
  );
};
