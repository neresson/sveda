export function humanizeModelId(raw: string): string {
  const part = raw.includes('/') ? (raw.split('/').pop() as string) : raw;
  const cleaned = part.replace(/-preview$/i, '').replace(/_/g, '-');
  return cleaned
    .split('-')
    .map(seg => {
      if (/^\d+(\.\d+)?$/.test(seg)) {
        return seg;
      }
      if (seg.length === 0) {
        return '';
      }
      return seg[0].toUpperCase() + seg.slice(1).toLowerCase();
    })
    .join(' ');
}

export function getSvedaModelDisplayName(model: string | null | undefined): string {
  if (!model) {
    return '—';
  }

  return humanizeModelId(model);
}
