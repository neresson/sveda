import type { JSX } from 'solid-js';
import { Show, type Component } from 'solid-js';
import { SvedaComposer } from './SvedaComposer';
import { SvedaQuickPrompts, type SvedaQuickPromptItem } from './SvedaQuickPrompts';

export interface SvedaWelcomeProps {
  inputMessage: string;
  onInputMessage: (value: string) => void;
  isLoading?: boolean;
  confirmationPending?: boolean;
  hasCurrentChat?: boolean;
  isStreaming?: boolean;
  showQuickPromptButtons?: boolean;
  quickPromptButtons?: SvedaQuickPromptItem[];
  quickPromptsTitle: string;
  onSend: () => void;
  onStop: () => void;
  onSendQuickPrompt: (prompt: string) => void;
  top?: JSX.Element;
}

export const SvedaWelcome: Component<SvedaWelcomeProps> = props => {
  return (
    <div class="flex min-h-0 flex-1 flex-col justify-center overflow-y-auto px-4 py-8">
      <div class="flex w-full flex-col gap-3">
        {props.top}
        <SvedaComposer
          value={props.inputMessage}
          onInput={props.onInputMessage}
          isLoading={props.isLoading}
          confirmationPending={props.confirmationPending}
          hasCurrentChat={props.hasCurrentChat}
          isStreaming={props.isStreaming}
          noBorder
          onSend={props.onSend}
          onStop={props.onStop}
        />
        <Show when={props.showQuickPromptButtons}>
          <SvedaQuickPrompts
            class="px-4"
            prompts={props.quickPromptButtons ?? []}
            title={props.quickPromptsTitle}
            disabled={props.isLoading || props.isStreaming || props.confirmationPending}
            onSelect={props.onSendQuickPrompt}
          />
        </Show>
      </div>
    </div>
  );
};
