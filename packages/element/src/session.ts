export interface SvedaSessionPayload {
  origin: string;
  token: string;
  appearance?: Record<string, unknown> | null;
}

export const parseSessionAttributes = (element: HTMLElement): {
  session: string | null;
  origin: string | null;
  token: string | null;
} => ({
  session: element.getAttribute('session')?.trim() || null,
  origin: element.getAttribute('origin')?.trim() || null,
  token: element.getAttribute('token')?.trim() || null,
});

export const resolveCsrfToken = (): string => {
  const meta = document.querySelector('meta[name="csrf-token"]');
  return meta?.getAttribute('content')?.trim() || '';
};

export const requestHostSession = async (sessionUrl: string): Promise<SvedaSessionPayload | null> => {
  const headers: Record<string, string> = {
    Accept: 'application/json',
    'X-Requested-With': 'XMLHttpRequest',
  };

  const csrf = resolveCsrfToken();
  if (csrf) {
    headers['X-CSRF-TOKEN'] = csrf;
  }

  try {
    const response = await fetch(sessionUrl, {
      method: 'POST',
      credentials: 'same-origin',
      headers,
    });

    if (!response.ok) {
      return null;
    }

    const payload = (await response.json()) as Partial<SvedaSessionPayload>;
    if (!payload.origin || !payload.token) {
      return null;
    }

    return {
      origin: payload.origin,
      token: payload.token,
      appearance: payload.appearance ?? null,
    };
  } catch {
    return null;
  }
};

export const resolveSvedaSession = async (element: HTMLElement): Promise<SvedaSessionPayload | null> => {
  const { session, origin, token } = parseSessionAttributes(element);

  if (origin && token) {
    return { origin, token };
  }

  if (!session) {
    return null;
  }

  return requestHostSession(session);
};
