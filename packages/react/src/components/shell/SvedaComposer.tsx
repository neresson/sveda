import { Loader2, Send, Square } from 'lucide-react';
import { useEffect, useRef, type KeyboardEvent } from 'react';
import { useSvedaT } from '../../provider';
import { cn } from '../../lib/utils';

export interface SvedaComposerProps {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onStop?: () => void;
  isLoading?: boolean;
  isStreaming?: boolean;
  disabled?: boolean;
}

export function SvedaComposer({
  value,
  onChange,
  onSend,
  onStop,
  isLoading = false,
  isStreaming = false,
  disabled = false,
}: SvedaComposerProps) {
  const t = useSvedaT();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const composerDisabled = disabled || isLoading;
  const canSend = value.trim().length > 0;

  useEffect(() => {
    const el = textareaRef.current;
    if (!el) {
      return;
    }
    el.style.height = '60px';
    const next = Math.min(Math.max(el.scrollHeight, 60), 250);
    el.style.height = `${next}px`;
  }, [value]);

  const handleKeyPress = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      if (!composerDisabled && canSend && !isStreaming) {
        onSend();
      }
    }
  };

  return (
    <div className="flex flex-shrink-0 flex-col bg-background p-3">
      <div className="sveda-chat-frame relative flex flex-col bg-background transition-colors">
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyPress={handleKeyPress}
          placeholder={t('typeMessage')}
          disabled={composerDisabled}
          className="sveda-chat-input-textarea max-h-[250px] min-h-[60px] w-full resize-none border-0 bg-transparent p-3 font-sans text-sm text-foreground shadow-none !outline-none !ring-0 placeholder:font-mono placeholder:text-[11px] placeholder:tracking-[0.08em] placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0"
          style={{ height: 60, outline: 'none', boxShadow: 'none' }}
          data-testid="sveda-composer"
        />

        <div className="flex items-center justify-between gap-2 border-t border-border/40 px-2 py-1.5">
          <div className="min-w-0 flex-1" />
          <div className="flex items-center gap-1">
            {isStreaming ? (
              <button
                type="button"
                className="inline-flex h-9 w-9 items-center justify-center text-muted-foreground hover:text-foreground"
                onClick={onStop}
                aria-label="Stop"
              >
                <Square className="h-4 w-4" />
              </button>
            ) : (
              <button
                type="button"
                className={cn(
                  'inline-flex h-9 w-9 items-center justify-center text-muted-foreground hover:text-foreground',
                  composerDisabled || !canSend ? 'opacity-40' : ''
                )}
                disabled={composerDisabled || !canSend}
                onClick={onSend}
                aria-label="Send"
                data-testid="sveda-send"
              >
                {isLoading ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Send className="h-4 w-4" />
                )}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
