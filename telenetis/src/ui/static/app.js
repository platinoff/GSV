const WS_URL = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`;

/* ---- Telegram-native layer (band 215, plan P1) ---- */

let BackButtonCalled = false;
let haptic = null;

function telegram() {
    return (window.Telegram && window.Telegram.WebApp) ? window.Telegram.WebApp : null;
}

/* P1a: whereAmI() — platform/module detection + branch body class.
   Telegram.WebApp.platform is one of: ios, android, macos, windows, linux, web. */
function whereAmI() {
    const tg = telegram();
    const platform = (tg && tg.platform) ? String(tg.platform).toLowerCase() : 'web';
    let cls;
    if (platform.startsWith('ios') || platform === 'iphone' || platform === 'ipad') cls = 'platform-ios';
    else if (platform.startsWith('android')) cls = 'platform-android';
    else if (platform.startsWith('macos') || platform === 'mac') cls = 'platform-macos';
    else if (platform.startsWith('windows')) cls = 'platform-windows';
    else if (platform.startsWith('linux')) cls = 'platform-linux';
    else cls = 'platform-web';

    const body = document.body;
    if (body) {
        body.classList.remove(
            'platform-ios', 'platform-android', 'platform-macos',
            'platform-windows', 'platform-linux', 'platform-web', 'platform-unknown'
        );
        body.classList.add(cls);
    }
    return cls;
}

/* P1g/b: theme — apply --tg-theme-* as CSS custom properties, handle light+dark
   via themeChanged. Values come from the SDK, so they are already safe colors,
   but we still route them through CSS variables only. */
function applyTheme() {
    const tg = telegram();
    const body = document.body;
    if (!tg || !body) return;
    const p = (tg.colorScheme === 'dark') ? 'dark' : 'light';
    body.classList.toggle('tg-dark', p === 'dark');
    body.classList.toggle('tg-light', p === 'light');

    const vars = {
        'bg_color': tg.themeParams.bg_color,
        'secondary_bg_color': tg.themeParams.secondary_bg_color,
        'text_color': tg.themeParams.text_color,
        'hint_color': tg.themeParams.hint_color,
        'link_color': tg.themeParams.link_color,
        'button_color': tg.themeParams.button_color,
        'button_text_color': tg.themeParams.button_text_color,
        'header_bg_color': tg.themeParams.header_bg_color,
        'section_bg_color': tg.themeParams.section_bg_color,
        'separator_color': tg.themeParams.section_separator_color,
    };
    for (const key of Object.keys(vars)) {
        const v = vars[key];
        if (v) body.style.setProperty(`--tg-theme-${key}`, v);
    }
    if (tg.setHeaderColor) tg.setHeaderColor(tg.themeParams.header_bg_color || '#1c2733');
    if (tg.setBackgroundColor) tg.setBackgroundColor(tg.themeParams.bg_color || '#0d1117');
    if (tg.ready) tg.ready();
}

/* P1c: haptics — a thin safe wrapper; selectionChanged/impactOccurred/notification
   are no-ops where unsupported, so we never throw. */
function haptics(kind) {
    const tg = telegram();
    if (!tg || !tg.HapticFeedback) return;
    try {
        if (kind === 'claim' || kind === 'done') tg.HapticFeedback.impactOccurred('medium');
        if (kind === 'error') tg.HapticFeedback.notificationOccurred('error');
        if (kind === 'nav') tg.HapticFeedback.selectionChanged();
    } catch (e) { /* ignore */ }
}

/* P1c: native BackButton — show it on non-root routes and pop history. */
function setupBackButton() {
    const tg = telegram();
    if (!tg || !tg.BackButton) return;
    const onRoot = window.location.pathname === '/' || window.location.pathname === '/app';
    if (onRoot) {
        tg.BackButton.hide();
    } else {
        tg.BackButton.show();
        if (!BackButtonCalled) {
            BackButtonCalled = true;
            tg.BackButton.onClick(function () {
                haptics('nav');
                history.length > 1 ? history.back() : (window.location.href = '/');
            });
        }
    }
}

/* P1d: safe-area sizing via --tg-viewport-stable-height, never rely on 100vh.
   Also expose the insets so CSS can pad the notch/gesture bar. */
function setupSafeArea() {
    const tg = telegram();
    const body = document.body;
    if (!tg || !body) return;
    const setStable = function () {
        if (tg.viewportStableHeight) {
            body.style.setProperty('--tg-viewport-stable-height', `${tg.viewportStableHeight}px`);
        }
    };
    if (tg.onEvent && tg.isVersionAtLeast) {
        if (tg.isVersionAtLeast('7.0')) {
            tg.onEvent('viewportChanged', setStable);
        }
    }
    setStable();
}

/* P1e: i18n — pull the string table from the Rust endpoint keyed by language,
   then fill every [data-i18n] node. No user-visible text is hardcoded in JS. */
async function applyI18n() {
    const tg = telegram();
    let lang = (tg && tg.initDataUnsafe && tg.initDataUnsafe.user && tg.initDataUnsafe.user.language_code) || 'en';
    try {
        const resp = await fetch(`/api/mini-app/i18n?lang=${encodeURIComponent(lang)}`);
        const data = await resp.json();
        const strings = data.strings || {};
        document.querySelectorAll('[data-i18n]').forEach(function (el) {
            const key = el.getAttribute('data-i18n');
            if (key && strings[key]) el.textContent = strings[key];
        });
        document.documentElement.lang = data.lang || 'en';
    } catch (e) {
        console.error('i18n load failed:', e);
    }
}

/* ---- live stream (unchanged behaviour, P2 backoff lands in band 216) ---- */

function connectWS() {
    const ws = new WebSocket(WS_URL);
    const log = document.getElementById('flow-log');

    ws.onopen = function () { if (log) log.classList.remove('ws-offline'); };
    ws.onmessage = function (event) {
        const data = JSON.parse(event.data);
        if (log) {
            const entry = document.createElement('div');
            entry.className = 'flow-entry';
            entry.innerHTML = `<span class="flow-ts">${data.ts}</span><strong>${data.jail_id}</strong> ${data.action}: ${data.detail}`;
            log.prepend(entry);
        }
    };
    ws.onclose = function () {
        if (log) log.classList.add('ws-offline');
        setTimeout(connectWS, 3000);
    };
}

async function loadStatus() {
    try {
        const resp = await fetch('/api/status');
        const data = await resp.json();
        const el = document.getElementById('status');
        if (el) {
            el.textContent = `${data.online ? t('status.online') : t('status.offline')} | ${t('status.tickets')}: ${data.tickets_count} | ${t('status.workers')}: ${data.workers_online}`;
        }
    } catch (e) {
        console.error('Failed to load status:', e);
    }
}

/* a tiny local fallback for the status line while /api/mini-app/i18n loads */
function t(key) {
    const map = {
        'status.online': 'Online',
        'status.offline': 'Offline',
        'status.tickets': 'Tickets',
        'status.workers': 'Workers',
    };
    return map[key] || key;
}

document.addEventListener('DOMContentLoaded', function () {
    const tg = telegram();
    if (tg && tg.expand) tg.expand();
    if (tg && tg.setHeaderColor) tg.setHeaderColor('#1c2733');

    whereAmI();
    applyTheme();
    setupSafeArea();
    setupBackButton();
    applyI18n().then(function () { loadStatus(); });

    if (tg && tg.onEvent && tg.isVersionAtLeast && tg.isVersionAtLeast('7.0')) {
        tg.onEvent('themeChanged', applyTheme);
    }

    connectWS();
    setInterval(loadStatus, 30000);
});
