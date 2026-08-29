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

/* P1e: i18n — strings come from the Rust endpoint (en/uk/ru, en fallback).
   The snapshot hydration carries the table so a cold start needs no extra
   round-trip; `fetchI18n` is only a fallback when the snapshot is unavailable. */
let i18nStrings = null;

function applyI18nStrings(strings) {
    if (!strings) return;
    i18nStrings = strings;
    document.querySelectorAll('[data-i18n]').forEach(function (el) {
        if (el.hasAttribute('data-skip-i18n')) return;
        const key = el.getAttribute('data-i18n');
        if (key && strings[key]) el.textContent = strings[key];
    });
}

async function fetchI18n() {
    const tg = telegram();
    const lang = (tg && tg.initDataUnsafe && tg.initDataUnsafe.user && tg.initDataUnsafe.user.language_code) || 'en';
    try {
        const resp = await fetch(`/api/mini-app/i18n?lang=${encodeURIComponent(lang)}`);
        const data = await resp.json();
        applyI18nStrings(data.strings || {});
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

function applyLiveConfig(data) {
    if (!data) return;
    liveConfig = Object.assign({}, liveConfig, {
        reconnect: Object.assign({}, liveConfig.reconnect, (data && data.reconnect) || {}),
        keepalive_secs: (data && data.keepalive_secs) || liveConfig.keepalive_secs,
    });
}

async function loadLiveConfig() {
    try {
        const resp = await fetch('/api/live/config');
        if (resp.ok) {
            const data = await resp.json();
            applyLiveConfig(data);
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

/* String lookups resolved from the snapshot i18n table when present, else a
   tiny English fallback (so the status line never shows a raw key). */
function t(key) {
    if (i18nStrings && i18nStrings[key]) return i18nStrings[key];
    const map = {
        'app.title': 'Telenetis',
        'status.online': 'Online',
        'status.offline': 'Offline',
        'status.tickets': 'Tickets',
        'status.workers': 'Workers',
        'board.empty': 'No tickets on the board.',
        'board.actions': 'Actions',
        'board.detail': 'Details',
        'board.no_description': 'No description.',
        'board.no_actions': 'No actions',
        'board.offline': 'Board unavailable — reconnecting…',
        'workers.none': 'No workers online.',
        'action.claim': 'Claim',
        'action.done': 'Done',
        'action.error': 'Error',
        'action.reclaim': 'Release',
        'action.claiming': 'Claiming...',
        'action.doing': 'Marking done...',
        'action.erroring': 'Flagging error...',
        'action.reclaiming': 'Releasing...',
        'action.claimed': 'Claimed',
        'action.done_ok': 'Done',
        'action.error_ok': 'Error flagged',
        'action.reclaim_ok': 'Released',
        'status.open': 'Open',
        'status.in_progress': 'In progress',
        'status.done': 'Done',
        'status.blocked': 'Blocked',
        'status.closed': 'Closed',
    };
    return map[key] || key;
}

/* Map a ticket status string to its display label. Falls back to the raw
   status when unknown so the board never shows a blank cell. */
function statusLabel(status) {
    const key = 'status.' + String(status || '');
    const label = t(key);
    return (key === label) ? (status || '') : label;
}

/* ---- cold start (band 217, plan P3) ----
   A cold Telegram WebView must paint structure instantly and feel live in
   under ~2s. Strategy:
     1. The templates ship skeleton screens (shimmer rows) — first paint is
        never blank.
     2. `bootstrap()` opens the WS immediately and fetches ONE consolidated
        `/api/snapshot` bundle (status + tickets + flows + workers + i18n +
        live config) — a single round-trip instead of five sequential ones.
     3. Hydration swaps skeleton → real content the moment the snapshot lands;
        the WS (now already open, or the SSE fallback) keeps it live from then on. */

function clearArea(el, area) {
    if (!el) return;
    el.querySelectorAll('[data-skeleton]').forEach(function (node) {
        node.remove();
    });
    el.removeAttribute('aria-busy');
}

function hydrateStatus(st) {
    const el = document.getElementById('status');
    if (!el) return;
    el.setAttribute('data-skip-i18n', '1');
    el.textContent = `${st.online ? t('status.online') : t('status.offline')} | ${t('status.tickets')}: ${st.tickets_count} | ${t('status.workers')}: ${st.workers_online}`;
}

function renderTicketRows(tickets) {
    const tbody = document.getElementById('board-body');
    if (!tbody) return;
    clearArea(tbody, 'board');
    if (!tickets || !tickets.length) {
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.colSpan = 6;
        td.className = 'board-empty';
        tr.appendChild(td);
        // No tickets at all → empty; otherwise hydration simply hasn't landed
        // (a skeleton/offline state) — the offline message covers the latter.
        td.textContent = t('board.empty');
        tbody.appendChild(tr);
        return;
    }
    tickets.forEach(function (tk) {
        const tr = document.createElement('tr');
        tr.setAttribute('data-ticket', tk.id || '');
        renderBoardRowData(tr, tk);
        tr.appendChild(actionButtonsCell(tk));
        tbody.appendChild(tr);
        // Expandable detail row (ticket notes / description).
        const detail = document.createElement('tr');
        detail.className = 'ticket-detail';
        detail.hidden = true;
        const detailTd = document.createElement('td');
        detailTd.colSpan = 6;
        const bodyText = (tk.body && String(tk.body).trim()) ? tk.body : t('board.no_description');
        detailTd.textContent = bodyText;
        detail.appendChild(detailTd);
        tbody.appendChild(detail);
    });
}

/* Render the five data cells of a board row (ID, Title, Status, Product,
   Claimed By) to match the 6-column header; actions go in a 6th cell. The
   title cell toggles the expandable detail row below. */
function renderBoardRowData(tr, tk) {
    const cols = [
        { key: 'id', cls: 'board-id' },
        { key: 'title', cls: 'board-title' },
        { key: 'status', cls: 'board-status' },
        { key: 'product', cls: 'board-product' },
        { key: 'claimed_by', cls: 'board-claimed' },
    ];
    cols.forEach(function (c, i) {
        const td = document.createElement('td');
        td.className = c.cls;
        if (c.key === 'status') {
            td.className += ' status-' + (tk.status || 'open');
            td.textContent = statusLabel(tk.status);
        } else {
            td.textContent = tk[c.key] != null ? String(tk[c.key]) : '';
        }
        if (c.key === 'title') {
            const toggle = document.createElement('button');
            toggle.type = 'button';
            toggle.className = 'detail-toggle';
            toggle.textContent = t('board.detail');
            toggle.addEventListener('click', function () {
                const detail = tr.nextElementSibling;
                if (detail) {
                    detail.hidden = !detail.hidden;
                    haptics('nav');
                }
            });
            td.appendChild(toggle);
        }
        tr.appendChild(td);
    });
}

/* ---- board action buttons (band 218, plan P4; band 219, ticket lifecycle) ----
   Each row offers the *server-authoritative* action set for its status
   (`tk.actions` from the snapshot wire): `open` → Claim; `in_progress` →
   Done / Error / Release; terminal statuses → none. The click POSTs the
   Telegram `initData` handshake (server-side HMAC-verified in Rust) and the
   ticket id to /api/board/{verb}; telenetis forwards to GSV. While in flight
   the button shows its busy label and is disabled; success toasts + haptic,
   failure restores the label and haptic-errors. */

function makeActionButton(action, ticketId) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'board-btn board-' + action;
    btn.textContent = t(actionLabel(action, false));
    btn.addEventListener('click', function () {
        postBoardAction(action, ticketId, btn);
    });
    return btn;
}

function actionButtonsCell(tk) {
    const td = document.createElement('td');
    td.className = 'board-actions';
    const actions = (tk && Array.isArray(tk.actions)) ? tk.actions : [];
    if (!actions.length) {
        td.className = 'board-actions board-actions-none';
        td.textContent = t('board.no_actions');
        return td;
    }
    actions.forEach(function (action) {
        td.appendChild(makeActionButton(action, tk.id));
    });
    return td;
}

function actionLabel(action, ok) {
    const busy = { claim: 'action.claiming', done: 'action.doing', error: 'action.erroring', reclaim: 'action.reclaiming' };
    const done = { claim: 'action.claimed', done: 'action.done_ok', error: 'action.error_ok', reclaim: 'action.reclaim_ok' };
    const plain = { claim: 'action.claim', done: 'action.done', error: 'action.error', reclaim: 'action.reclaim' };
    return t(ok ? done[action] : (busy[action] || plain[action]));
}

async function postBoardAction(action, ticketId, btn) {
    const tg = telegram();
    const initData = (tg && tg.initData) ? tg.initData : '';
    const authDate = Math.floor(Date.now() / 1000);
    btn.disabled = true;
    btn.textContent = actionLabel(action, false);
    try {
        const resp = await fetch(`/api/board/${action}?initData=${encodeURIComponent(initData)}&authDate=${authDate}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: ticketId }),
        });
        const data = await resp.json();
        if (data && data.ok) {
            haptics('done');
            btn.textContent = actionLabel(action, true);
            btn.classList.add('board-btn-ok');
        } else {
            throw new Error((data && data.error) || ('HTTP ' + resp.status));
        }
    } catch (e) {
        btn.disabled = false;
        btn.textContent = actionLabel(action, false);
        haptics('error');
        console.error('board action failed:', e);
    }
}

function renderTickets(tickets) {
    const el = document.getElementById('tickets');
    if (!el) return;
    clearArea(el, 'tickets');
    if (!tickets.length) {
        el.textContent = t('board.empty');
        return;
    }
    const list = document.createElement('ul');
    list.className = 'ticket-list';
    tickets.forEach(function (tk) {
        const li = document.createElement('li');
        li.className = 'ticket-item';
        const badge = document.createElement('span');
        badge.className = 'status-badge status-' + (tk.status || 'open');
        badge.textContent = tk.status || '';
        li.textContent = `${tk.id} — ${tk.title}`;
        li.prepend(badge);
        list.appendChild(li);
    });
    el.appendChild(list);
}

function renderWorkers(workers) {
    const el = document.getElementById('workers');
    if (!el) return;
    clearArea(el, 'workers');
    if (!workers.length) {
        el.textContent = t('workers.none');
        return;
    }
    const list = document.createElement('ul');
    list.className = 'worker-list';
    workers.forEach(function (w) {
        const li = document.createElement('li');
        li.className = 'worker-item';
        const dot = document.createElement('span');
        dot.className = 'worker-dot status-' + (w.status || 'offline');
        li.textContent = `${w.jail_id} — ${w.ide} L${w.rank}`;
        li.prepend(dot);
        list.appendChild(li);
    });
    el.appendChild(list);
}

function renderRoles(workers) {
    const el = document.getElementById('roles');
    if (!el) return;
    clearArea(el, 'roles');
    if (!workers.length) {
        el.textContent = t('workers.none');
        return;
    }
    const list = document.createElement('ul');
    list.className = 'worker-list';
    workers.slice().sort(function (a, b) { return (b.rank || 0) - (a.rank || 0); })
        .forEach(function (w) {
            const li = document.createElement('li');
            li.className = 'role-item';
            li.textContent = `L${w.rank} — ${w.jail_id} · ${w.agent} · ${w.ide} [${w.status}]`;
            list.appendChild(li);
        });
    el.appendChild(list);
}

function renderFlows(flows) {
    const log = flowLog();
    if (!log) return;
    clearArea(log, 'flow-log');
    (flows || []).forEach(function (data) { renderFlow(data); });
}

function hydrateFromSnapshot(s) {
    if (!s) return;
    if (s.i18n) {
        applyI18nStrings(s.i18n.strings || {});
        document.documentElement.lang = s.i18n.lang || 'en';
    }
    if (s.live) applyLiveConfig(s.live);
    if (s.status) hydrateStatus(s.status);
    if (Array.isArray(s.tickets)) {
        renderTickets(s.tickets);
        renderTicketRows(s.tickets);
    }
    if (Array.isArray(s.workers)) {
        renderWorkers(s.workers);
        renderRoles(s.workers);
    }
    if (Array.isArray(s.flows)) renderFlows(s.flows);
    if (s.status && s.status.jail_id) {
        document.body.classList.add('jail-' + String(s.status.jail_id).replace(/[^a-z0-9-]/gi, ''));
    }
}

async function fetchSnapshot(lang) {
    const resp = await fetch(`/api/snapshot?lang=${encodeURIComponent(lang || 'en')}`);
    if (!resp.ok) throw new Error('snapshot ' + resp.status);
    return resp.json();
}

async function bootstrap() {
    const tg = telegram();
    const lang = (tg && tg.initDataUnsafe && tg.initDataUnsafe.user && tg.initDataUnsafe.user.language_code) || 'en';

    // Early WS upgrade: open the live channel in parallel with the snapshot
    // prefetch, so the first live events can stream the instant hydration lands.
    connectWS();

    try {
        hydrateFromSnapshot(await fetchSnapshot(lang));
    } catch (e) {
        // Offline cold start: keep the skeletons visible and rely on the
        // standalone fallbacks (i18n + live config endpoints) so the board is
        // never a frozen blank page.
        console.error('snapshot failed:', e);
        setBoardOffline();
        fetchI18n();
        loadLiveConfig();
    }
    loadStatus();
}

/* Offline/error empty state for the board: swap the rows for a single
   reconnecting notice so a worker a cold WebView can't mistake a lack of data
   for an empty board. */
function setBoardOffline() {
    const tbody = document.getElementById('board-body');
    if (!tbody) return;
    clearArea(tbody, 'board');
    const tr = document.createElement('tr');
    const td = document.createElement('td');
    td.colSpan = 6;
    td.className = 'board-empty board-offline';
    td.textContent = t('board.offline');
    tr.appendChild(td);
    tbody.appendChild(tr);
}

document.addEventListener('DOMContentLoaded', function () {
    const tg = telegram();
    if (tg && tg.expand) tg.expand();
    if (tg && tg.setHeaderColor) tg.setHeaderColor('#1c2733');

    whereAmI();
    applyTheme();
    setupSafeArea();
    setupBackButton();

    if (tg && tg.onEvent && tg.isVersionAtLeast && tg.isVersionAtLeast('7.0')) {
        tg.onEvent('themeChanged', applyTheme);
    }

    bootstrap();
    setInterval(loadStatus, 30000);
});
