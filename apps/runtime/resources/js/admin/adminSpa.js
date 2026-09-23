export const adminPathnames = (urls) =>
    Object.values(urls ?? {})
        .filter((url) => typeof url === 'string' && url !== '')
        .map((url) => new URL(url, window.location.origin).pathname);

export const isAdminSpaUrl = (href, urls) => {
    try {
        const url = new URL(href, window.location.origin);

        return url.origin === window.location.origin && adminPathnames(urls).includes(url.pathname);
    } catch {
        return false;
    }
};

export const shouldInterceptAdminClick = (event, urls) => {
    if (
        event.defaultPrevented
        || event.button !== 0
        || event.metaKey
        || event.ctrlKey
        || event.shiftKey
        || event.altKey
    ) {
        return false;
    }

    const link = event.target instanceof Element ? event.target.closest('a[href]') : null;
    if (!link || link.target === '_blank' || link.hasAttribute('download')) {
        return false;
    }

    const href = link.getAttribute('href') ?? '';
    if (href.startsWith('#') || href.startsWith('mailto:') || href.startsWith('tel:')) {
        return false;
    }

    return isAdminSpaUrl(link.href, urls);
};
