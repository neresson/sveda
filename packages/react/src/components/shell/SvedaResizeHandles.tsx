import type { PointerEvent } from 'react';
import type { SvedaChatViewMode } from '../../hooks/useSvedaChatLayout';

export interface SvedaResizeHandlesProps {
  isMobile: boolean;
  viewMode: SvedaChatViewMode;
  onStartResize: (direction: string, event: PointerEvent<HTMLElement>) => void;
}

export function SvedaResizeHandles({ isMobile, viewMode, onStartResize }: SvedaResizeHandlesProps) {
  if (isMobile || (viewMode !== 'fixed' && viewMode !== 'floating')) {
    return null;
  }

  if (viewMode === 'fixed') {
    return (
      <div
        className="absolute bottom-3 left-0 top-3 z-10 w-1 touch-none cursor-ew-resize hover:bg-primary/30"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize chat"
        onPointerDown={(event) => onStartResize('fixed-left', event)}
      />
    );
  }

  return (
    <>
      <div
        className="absolute left-0 top-0 z-10 h-3 w-3 touch-none cursor-nw-resize rounded-br hover:bg-primary/20"
        role="separator"
        aria-label="Resize chat"
        onPointerDown={(event) => onStartResize('top-left', event)}
      />
      <div
        className="absolute left-3 right-3 top-0 z-10 h-1 touch-none cursor-n-resize hover:bg-primary/30"
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize chat height"
        onPointerDown={(event) => onStartResize('top', event)}
      />
      <div
        className="absolute bottom-3 left-0 top-3 z-10 w-1 touch-none cursor-w-resize hover:bg-primary/30"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize chat width"
        onPointerDown={(event) => onStartResize('left', event)}
      />
    </>
  );
}
