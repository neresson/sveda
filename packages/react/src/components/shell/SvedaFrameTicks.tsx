export function SvedaFrameTicks() {
  return (
    <>
      <span
        className="pointer-events-none absolute -left-px -top-px z-10 h-2.5 w-2.5 border-l border-t border-foreground"
        aria-hidden="true"
      />
      <span
        className="pointer-events-none absolute -right-px -top-px z-10 h-2.5 w-2.5 border-r border-t border-foreground"
        aria-hidden="true"
      />
      <span
        className="pointer-events-none absolute -bottom-px -left-px z-10 h-2.5 w-2.5 border-b border-l border-foreground"
        aria-hidden="true"
      />
      <span
        className="pointer-events-none absolute -bottom-px -right-px z-10 h-2.5 w-2.5 border-b border-r border-foreground"
        aria-hidden="true"
      />
    </>
  );
}
