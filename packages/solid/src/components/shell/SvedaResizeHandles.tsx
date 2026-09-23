import { Show, type Component } from 'solid-js';

export interface SvedaResizeHandlesProps {
  isMobile: boolean;
  viewMode: string;
  onStartResize: (direction: string, event: PointerEvent) => void;
}

export const SvedaResizeHandles: Component<SvedaResizeHandlesProps> = props => {
  return (
    <Show when={!props.isMobile && (props.viewMode === 'fixed' || props.viewMode === 'floating')}>
      <Show
        when={props.viewMode === 'fixed'}
        fallback={
          <>
            <div
              class="absolute left-0 top-0 z-10 h-3 w-3 touch-none cursor-nw-resize rounded-br hover:bg-primary/20"
              role="separator"
              aria-label="Resize chat"
              onPointerDown={event => props.onStartResize('top-left', event)}
            />
            <div
              class="absolute left-3 right-3 top-0 z-10 h-1 touch-none cursor-n-resize hover:bg-primary/30"
              role="separator"
              aria-orientation="horizontal"
              aria-label="Resize chat height"
              onPointerDown={event => props.onStartResize('top', event)}
            />
            <div
              class="absolute bottom-3 left-0 top-3 z-10 w-1 touch-none cursor-w-resize hover:bg-primary/30"
              role="separator"
              aria-orientation="vertical"
              aria-label="Resize chat width"
              onPointerDown={event => props.onStartResize('left', event)}
            />
          </>
        }
      >
        <div
          class="absolute bottom-3 left-0 top-3 z-10 w-1 touch-none cursor-ew-resize hover:bg-primary/30"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize chat"
          onPointerDown={event => props.onStartResize('fixed-left', event)}
        />
      </Show>
    </Show>
  );
};
