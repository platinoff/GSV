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

/* ---- live stream primacy (band 216, plan P2) ----
   WS /ws is the primary live channel; /events SSE is the fallback after
   RECONNECT_MAX_ATTEMPTS consecutive WS failures. Both feeds render into the
   same #flow-log so a drop never blanks the board. The retry schedule comes
   from the server via /api/live/config and matches Rust stream::backoff. */

const LIVE_DEFAULTS = {
    reconnect: { base_ms: 1000, cap_ms: 30000, max_attempts: 6 },
    keepalive_secs: 25,
};
let liveConfig = LIVE_DEFAULTS;
let wsAttempts = 0;
let usingSse = false;
let sseSource = null;

async function loadLiveConfig() {
    try {
        const resp = await fetch('/api/live/config');
        if (resp.ok) {
            const data = await resp.json();
            liveConfig = Object.assign({}, LIVE_DEFAULTS, {
                reconnect: Object.assign({}, LIVE_DEFAULTS.reconnect, (data && data.reconnect) || {}),
                keepalive_secs: (data && data.keepalive_secs) || LIVE_DEFAULTS.keepalive_secs,
            });
        }
    } catch (e) { /* keep defaults */ }
}

/* Deterministic jitter in tenths (0..4), mirrors Rust jitter_tenths. */
function jitterTenths(seed, attempt) {
    const P1 = 0x9E3779B97F4A7C15, P2 = 0xBF58476D1CE4E5B9, P3 = 0x94D049BB133111EB;
    let z = (seed + ((attempt * P1) >>> 0)) >>> 0;
    z = ((z ^ (z >>> 30)) * P2) >>> 0;
    z = ((z ^ (z >>> 27)) * P3) >>> 0;
    z ^= z >>> 31;
    return (z % 5); // 0..4
}

/* Exponential backoff with jitter, mirrors Rust ReconnectPolicy::delay_ms. */
function wsDelayMs(attempt) {
    const p = liveConfig.reconnect;
    if (attempt < 1 || attempt > p.max_attempts) return null;
    const capped = Math.min(p.base_ms * Math.pow(2, attempt - 1), p.cap_ms);
    return capped + Math.floor((capped * jitterTenths(12345, attempt)) / 10);
}

function flowLog() {
    return document.getElementById('flow-log');
}

function setFeedClass(which) {
    const log = flowLog();
    if (!log) return;
    log.classList.remove('ws-offline', 'sse-fallback', 'ws-live', 'sse-live');
    if (which === 'ws') log.classList.add('ws-live');
    else if (which === 'sse') log.classList.add('sse-live', 'sse-fallback');
    else log.classList.add('ws-offline');
    // surface the active channel for CSS + a11y
    log.setAttribute('data-feed', which || 'offline');
}

function renderFlow(data) {
    const log = flowLog();
    if (!log) return;
    if (!data || data.type === 'ping') return;
    const entry = document.createElement('div');
    entry.className = 'flow-entry';
    entry.innerHTML = `<span class="flow-ts">${data.ts || ''}</span><strong>${data.jail_id || ''}</strong> ${data.action || ''}: ${data.detail || ''}`;
    log.prepend(entry);
}

function scheduleWSRetry() {
    wsAttempts += 1;
    if (usingSse) return;
    if (wsAttempts > liveConfig.reconnect.max_attempts) {
        connectSSE();
        return;
    }
    const delay = wsDelayMs(wsAttempts) != null ? wsDelayMs(wsAttempts) : 3000;
    setTimeout(connectWS, delay);
}

function connectWS() {
    if (usingSse) return;
    let ws;
    try { ws = new WebSocket(WS_URL); } catch (e) { scheduleWSRetry(); return; }

    ws.onopen = function () {
        wsAttempts = 0;
        setFeedClass('ws');
    };
    ws.onmessage = function (event) {
        let data;
        try { data = JSON.parse(event.data); } catch (e) { return; }
        renderFlow(data);
    };
    ws.onclose = function () {
        setFeedClass('offline');
        scheduleWSRetry();
    };
    ws.onerror = function () {
        try { ws.close(); } catch (e) { /* fall through to onclose */ }
    };
}

function connectSSE() {
    if (usingSse) return;
    if (sseSource) { setFeedClass('sse'); return; }
    usingSse = true;
    setFeedClass('sse');
    const es = new EventSource('/events');
    sseSource = es;
    es.onopen = function () { setFeedClass('sse'); };
    es.onmessage = function (event) {
        let data;
        try { data = JSON.parse(event.data); } catch (e) { return; }
        renderFlow(data);
    };
    es.onerror = function () {
        // Leave reconnection to the browser's native SSE retry; stay on the
        // fallback so we never bounce back to a flapping WS.
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

    loadLiveConfig().then(function () { connectWS(); });
    setInterval(loadStatus, 30000);
});
