export interface SvedaClientCapabilities {
  allow: string[];
}

export interface SvedaMcpCapabilities {
  allow: string[];
  domains?: string[];
  max_mode?: string;
  maxMode?: string;
}

export interface SvedaCapabilities {
  restricted: boolean;
  web?: boolean;
  code?: boolean;
  mcp?: SvedaMcpCapabilities;
  client?: SvedaClientCapabilities;
}

export function globMatch(value: string, pattern: string): boolean {
  const trimmed = pattern.trim();
  if (trimmed === '') {
    return false;
  }
  if (trimmed === '*') {
    return true;
  }
  if (trimmed.endsWith('*')) {
    const prefix = trimmed.slice(0, -1);
    return prefix === '' || value.startsWith(prefix);
  }
  if (trimmed.startsWith('*')) {
    return value.endsWith(trimmed.slice(1));
  }
  return value === trimmed;
}

export function parseSvedaCapabilities(value: unknown): SvedaCapabilities | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null;
  }

  const record = value as Record<string, unknown>;
  const restricted = Boolean(record.restricted);
  const clientRaw = record.client;
  const mcpRaw = record.mcp;

  const client =
    clientRaw && typeof clientRaw === 'object' && !Array.isArray(clientRaw)
      ? {
          allow: Array.isArray((clientRaw as Record<string, unknown>).allow)
            ? ((clientRaw as Record<string, unknown>).allow as unknown[])
                .map((item) => String(item).trim())
                .filter(Boolean)
            : [],
        }
      : undefined;

  const mcp =
    mcpRaw && typeof mcpRaw === 'object' && !Array.isArray(mcpRaw)
      ? {
          allow: Array.isArray((mcpRaw as Record<string, unknown>).allow)
            ? ((mcpRaw as Record<string, unknown>).allow as unknown[])
                .map((item) => String(item).trim())
                .filter(Boolean)
            : [],
          domains: Array.isArray((mcpRaw as Record<string, unknown>).domains)
            ? ((mcpRaw as Record<string, unknown>).domains as unknown[])
                .map((item) => String(item).trim())
                .filter(Boolean)
            : undefined,
          max_mode:
            typeof (mcpRaw as Record<string, unknown>).max_mode === 'string'
              ? String((mcpRaw as Record<string, unknown>).max_mode)
              : typeof (mcpRaw as Record<string, unknown>).maxMode === 'string'
                ? String((mcpRaw as Record<string, unknown>).maxMode)
                : undefined,
        }
      : undefined;

  return {
    restricted,
    web: typeof record.web === 'boolean' ? record.web : undefined,
    code: typeof record.code === 'boolean' ? record.code : undefined,
    mcp,
    client,
  };
}

export function clientToolAllowed(name: string, capabilities: SvedaCapabilities | null): boolean {
  if (!capabilities?.restricted) {
    return true;
  }

  const allow = capabilities.client?.allow ?? [];
  if (allow.length === 0) {
    return false;
  }

  return allow.some((pattern) => globMatch(name, pattern));
}
