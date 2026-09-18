(function () {
  const scriptTag = document.currentScript || (function () {
    const scripts = document.getElementsByTagName('script');
    return scripts[scripts.length - 1];
  })();

  const sessionUrl = (scriptTag.getAttribute('data-session') || '').trim();
  if (!sessionUrl) {
    console.error('Sveda widget: data-session is missing on the script tag.');
    return;
  }

  const container = document.createElement('div');
  container.id = 'sveda-widget-container';
  container.style.position = 'fixed';
  container.style.bottom = '0';
  container.style.right = '0';
  container.style.left = 'auto';
  container.style.top = 'auto';
  container.style.width = '240px';
  container.style.overflow = 'hidden';
  container.style.zIndex = '999999';
  container.style.pointerEvents = 'auto';

  const iframe = document.createElement('iframe');
  iframe.style.width = '100%';
  iframe.style.height = '48px';
  iframe.style.minHeight = '48px';
  iframe.style.border = 'none';
  iframe.style.backgroundColor = 'transparent';
  iframe.style.display = 'block';
  iframe.setAttribute('title', 'Sveda');
  iframe.allow = 'clipboard-write';

  const applyMinimizedLayout = function () {
    container.style.left = 'auto';
    container.style.right = '0';
    container.style.top = 'auto';
    container.style.bottom = '0';
    container.style.width = '240px';
    container.style.minWidth = '0';
    container.style.height = '48px';
    container.style.minHeight = '48px';
    container.style.maxWidth = 'calc(100vw - 40px)';
    container.style.maxHeight = 'none';
    iframe.style.width = '100%';
    iframe.style.height = '48px';
    iframe.style.minHeight = '48px';
  };

  const clampSize = function (width, height) {
    const maxW = Math.max(320, window.innerWidth - 40);
    const maxH = Math.max(400, window.innerHeight - 40);
    const w = typeof width === 'number' && !isNaN(width) ? width : 400;
    const h = typeof height === 'number' && !isNaN(height) ? height : 600;
    return {
      width: Math.max(320, Math.min(w, maxW)),
      height: Math.max(400, Math.min(h, maxH)),
    };
  };

  const applyExpandedLayout = function (width, height) {
    const size = clampSize(width, height);
    container.style.left = 'auto';
    container.style.right = '0';
    container.style.top = 'auto';
    container.style.bottom = '0';
    container.style.width = size.width + 'px';
    container.style.height = size.height + 'px';
    container.style.maxWidth = 'calc(100vw - 40px)';
    container.style.maxHeight = 'calc(100vh - 40px)';
    iframe.style.width = '100%';
    iframe.style.height = '100%';
    iframe.style.minHeight = '0';
  };

  const applyFixedLayout = function (width) {
    const size = clampSize(width, window.innerHeight);
    container.style.left = 'auto';
    container.style.right = '0';
    container.style.top = '0';
    container.style.bottom = '0';
    container.style.width = size.width + 'px';
    container.style.height = '100%';
    container.style.minWidth = '0';
    container.style.minHeight = '0';
    container.style.maxWidth = '100vw';
    container.style.maxHeight = 'none';
    iframe.style.width = '100%';
    iframe.style.height = '100%';
    iframe.style.minHeight = '0';
  };

  const applyFullscreenLayout = function () {
    container.style.left = '0';
    container.style.right = '0';
    container.style.top = '0';
    container.style.bottom = '0';
    container.style.width = '100%';
    container.style.height = '100%';
    container.style.minWidth = '0';
    container.style.minHeight = '0';
    container.style.maxWidth = 'none';
    container.style.maxHeight = 'none';
    iframe.style.width = '100%';
    iframe.style.height = '100%';
    iframe.style.minHeight = '0';
  };

  applyMinimizedLayout();
  container.appendChild(iframe);
  document.body.appendChild(container);

  let opened = false;
  let iframeReady = false;

  const csrfToken = function () {
    const meta = document.querySelector('meta[name="csrf-token"]');
    return meta ? meta.getAttribute('content') || '' : '';
  };

  const openEmbed = async function () {
    if (opened) {
      return;
    }

    const headers = {
      Accept: 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
    };
    const csrf = csrfToken();
    if (csrf) {
      headers['X-CSRF-TOKEN'] = csrf;
    }

    const response = await fetch(sessionUrl, {
      method: 'POST',
      credentials: 'same-origin',
      headers,
    });
    const payload = await response.json().catch(function () {
      return {};
    });
    if (!response.ok || !payload.origin || !payload.token) {
      console.error('Sveda widget: failed to start session.', payload.message || response.status);
      return;
    }

    const embedUrl = new URL('/sveda/embed', payload.origin);
    embedUrl.searchParams.set('token', payload.token);
    iframe.src = embedUrl.toString();
    opened = true;
  };

  const applyResizeMessage = function (data) {
    if (!data || data.type !== 'sveda:resize') {
      return;
    }
    if (data.isMinimized) {
      applyMinimizedLayout();
      return;
    }
    if (data.immersive) {
      applyFullscreenLayout();
      return;
    }
    if (data.fixed) {
      applyFixedLayout(data.frameWidth);
      return;
    }
    applyExpandedLayout(data.frameWidth, data.frameHeight);
  };

  window.addEventListener('message', function (event) {
    if (!iframe.contentWindow || event.source !== iframe.contentWindow) {
      return;
    }
    const data = event.data;
    if (!data || typeof data !== 'object') {
      return;
    }
    if (data.type === 'sveda:ready') {
      iframeReady = true;
      return;
    }
    applyResizeMessage(data);
  });

  void openEmbed();
})();
