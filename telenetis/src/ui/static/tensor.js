/* Browser tensor worker (swarm path b, PoC ticket t-1789571785502387700).
 *
 * Torrent-style UI: every model is a row with its own state and buttons
 * (Download / Pause / Resume / Cancel / Use) — no single-select control,
 * nothing to mis-tap. Runs a GGUF IN THIS PHONE via wllama (llama.cpp
 * WASM + WebGPU, fastest current browser runtime: +54% decode vs WebLLM,
 * GGUF-native so our Qwen quants load unconverted), then serves
 * `llama_chat` tasks from this device's poolAI peer queue:
 *   poll GET /edge/upstream/poolai/api/v1/virtual-nodes/{peer}/tasks/poll
 *   run locally -> POST .../tasks/{id}/complete {status, detail}
 * Same origin through the Telenetis reverse proxy, so no CORS and no
 * bearer handling in the browser. Non-chat tasks are re-queued untouched
 * (poll pops — dropping them would lose PC-side work).
 *
 * OPFS is absent on HTTP LAN + old WebViews, so wllama gets a custom RAM
 * cache (plain fetch, no OPFS) and finished files persist to IndexedDB.
 * Downloads support pause/resume via Range, speed stats, WakeLock against
 * device sleep, and a stall watchdog that resumes dead streams.
 */
var WLLAMA_PIN = '3.5.1';
var N_GPU_LAYERS = 99; // all layers to WebGPU; CPU fallback on failure
var POLL_MS = 5000;
var MAX_REQUEUE_PER_TICK = 3;
var STALL_MS = 20000;

// Runtime + models resolve at boot from /api/edge/tensor/config
// (same-origin vendor + host GGUF library). HuggingFace pair below is the
// fallback when the host serves no catalog — needs internet + the CSP
// huggingface allowance, and the exact repo/file names.
var RT = { esm: '', wasm: '' };
var HF_RT = {
    esm: 'https://cdn.jsdelivr.net/npm/@wllama/wllama@3.5.1/esm/index.js',
    wasm: 'https://cdn.jsdelivr.net/npm/@wllama/wllama@3.5.1/esm/wasm/wllama.wasm'
};
var HF_MODELS = {
    qwen15: {
        label: 'Qwen2.5-1.5B Q4_K_M (~1GB, HF)',
        hf: true,
        repo: 'Qwen/Qwen2.5-1.5B-Instruct-GGUF',
        file: 'qwen2.5-1.5b-instruct-q4_k_m.gguf'
    },
    qwen05: {
        label: 'Qwen2.5-0.5B Q4_K_M (~400MB, HF fallback)',
        hf: true,
        repo: 'Qwen/Qwen2.5-0.5B-Instruct-GGUF',
        file: 'qwen2.5-0.5b-instruct-q4_k_m.gguf'
    }
};
var MODELS = {};

// Per-model download state: idle | active | paused | done | error.
var DLS = {};

// Shared byte store (RAM) across loads: filename -> {blob, size}.
var BYTESTORE = new Map();

var S = {
    wllama: null,
    modelKey: '',
    modelLabel: '',
    gpu: true,
    peer: '',
    running: false,
    wpaused: false,
    timer: null,
    busy: false,
    done: 0,
    rt: null // loaded runtime {Wllama, wasmUrl}
};

// Device persistence (IndexedDB works on HTTP LAN + old WebViews where
// OPFS is absent): finished downloads survive reloads; RAM is the
// fallback when IDB is missing or quota-rejected. Module-level so one
// open serves every Load.
var IDB = {
    ok: false,
    db: null,
    open: function () {
        var self = this;
        return new Promise(function (resolve) {
            try {
                if (!('indexedDB' in window)) { resolve(false); return; }
                var req = indexedDB.open('tensor-worker', 1);
                req.onupgradeneeded = function () {
                    try { req.result.createObjectStore('models'); } catch (e) {}
                };
                req.onsuccess = function () { self.db = req.result; self.ok = true; resolve(true); };
                req.onerror = function () { resolve(false); };
            } catch (e) { resolve(false); }
        });
    },
    get: function (name) {
        var self = this;
        return new Promise(function (resolve) {
            if (!self.ok) { resolve(null); return; }
            try {
                var tx = self.db.transaction('models', 'readonly');
                var rq = tx.objectStore('models').get(name);
                rq.onsuccess = function () { resolve(rq.result || null); };
                rq.onerror = function () { resolve(null); };
            } catch (e) { resolve(null); }
        });
    },
    put: function (name, blob) {
        var self = this;
        return new Promise(function (resolve) {
            if (!self.ok) { resolve(false); return; }
            try {
                var tx = self.db.transaction('models', 'readwrite');
                var rq = tx.objectStore('models').put({ blob: blob, size: blob.size }, name);
                rq.onsuccess = function () { resolve(true); };
                rq.onerror = function () { resolve(false); };
            } catch (e) { resolve(false); }
        });
    },
    listAll: function () {
        // Every cached model with its byte size (drives the [cached] tags,
        // so an empty device is visible instead of silent).
        var self = this;
        return new Promise(function (resolve) {
            if (!self.ok) { resolve([]); return; }
            try {
                var out = [];
                var tx = self.db.transaction('models', 'readonly');
                var rq = tx.objectStore('models').openCursor();
                rq.onsuccess = function () {
                    var cur = rq.result;
                    if (cur) {
                        var size = 0;
                        try { size = (cur.value && cur.value.size) || 0; } catch (e) {}
                        out.push({ name: cur.key, size: size });
                        cur.continue();
                    } else {
                        resolve(out);
                    }
                };
                rq.onerror = function () { resolve(out); };
            } catch (e) { resolve([]); }
        });
    }
};

function esc(s) {
    return String(s == null ? '' : s)
        .replace(/&/g, '&amp;').replace(/</g, '&lt;')
        .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function el(id) { return document.getElementById(id); }

function setStatus(html) {
    var e = el('tensor-status');
    if (e) { e.innerHTML = html; }
}

function logRow(who, text) {
    var body = el('tensor-log');
    if (!body) { return; }
    var tr = document.createElement('tr');
    var td1 = document.createElement('td');
    td1.textContent = who;
    var td2 = document.createElement('td');
    td2.textContent = String(text).slice(0, 500);
    tr.appendChild(td1);
    tr.appendChild(td2);
    body.insertBefore(tr, body.firstChild);
    while (body.rows.length > 30) { body.deleteRow(-1); }
}

function queryPeer() {
    try {
        var m = /[?&]peer=([^&]+)/.exec(location.search || '');
        return m ? decodeURIComponent(m[1]).trim() : '';
    } catch (e) { return ''; }
}

function lanHint() {
    // Tunnel fetches die on big files (free-tier interstitial/limits);
    // same Wi-Fi must go direct LAN. Derive the LAN host from the edge
    // endpoints (same box serves llama/poolai) and suggest it once.
    timed('/api/edge/endpoints', { headers: { Accept: 'application/json' } }, 15000)
        .then(function (r) { return r.json(); })
        .then(function (data) {
            var svcs = (data && data.services) || [];
            var host = '';
            for (var i = 0; i < svcs.length; i++) {
                var m = /http:\/\/([^:\/]+)/.exec(svcs[i].lan || '');
                if (m) { host = m[1]; break; }
            }
            if (!host || location.hostname === host) { return; }
            var peer = queryPeer();
            var url = 'http://' + host + ':9800/tensor' + (peer ? '?peer=' + encodeURIComponent(peer) : '');
            var box = el('tensor-lan');
            if (box) {
                box.innerHTML = 'Slow tunnel? Same Wi-Fi goes direct LAN: ' +
                    '<a href="' + esc(url) + '">' + esc(url) + '</a>';
            }
        })
        .catch(function () {});
}

function tgUser() {
    try {
        var w = window.Telegram && window.Telegram.WebApp;
        if (w && w.initDataUnsafe && w.initDataUnsafe.user && w.initDataUnsafe.user.id) {
            return String(w.initDataUnsafe.user.id);
        }
    } catch (e) {}
    return '';
}

// Screen wake lock: locked devices freeze WebView fetches mid-download,
// resuming to a dead stream. Held while downloading or working, released
// on stop/finish. Best-effort (old WebViews lack the API).
var WL = { lock: null, want: false };
function wakeLock(on) {
    WL.want = !!on;
    function release() {
        if (!WL.lock) { return; }
        try {
            var l = WL.lock;
            WL.lock = null;
            if (l.release) { l.release(); }
        } catch (e) {}
    }
    if (!on) { release(); return Promise.resolve(); }
    try {
        if (!('wakeLock' in navigator) || !navigator.wakeLock.request) {
            logRow('sys', 'wake lock unsupported — keep the screen on manually');
            return Promise.resolve();
        }
        return navigator.wakeLock.request('screen').then(function (l) {
            WL.lock = l;
            logRow('sys', 'screen lock held (download/worker)');
            l.addEventListener('release', function () {
                WL.lock = null;
                if (WL.want) {
                    logRow('sys', 'screen lock lost — re-request on return');
                }
            });
        }).catch(function (e) {
            logRow('sys', 'wake lock denied: ' + String((e && e.name) || e));
        });
    } catch (e) {
        return Promise.resolve();
    }
}
document.addEventListener('visibilitychange', function () {
    // Re-acquire after sleep if still needed; the stall watchdog below
    // resumes the byte stream itself.
    if (!document.hidden && WL.want && !WL.lock) { wakeLock(true); }
});

function timed(url, opts, ms) {
    if (typeof AbortController === 'undefined' || typeof fetch === 'undefined') {
        return fetch(url, opts);
    }
    var ctrl = new AbortController();
    var timer = setTimeout(function () { ctrl.abort(); }, ms || 30000);
    return fetch(url, Object.assign({}, opts, { signal: ctrl.signal }))
        .then(function (r) { clearTimeout(timer); return r; },
              function (e) { clearTimeout(timer); throw e; });
}

function pool(path, opts, ms) {
    return timed('/edge/upstream/poolai/api/v1' + path, opts, ms)
        .then(function (r) {
            if (!r.ok) { throw new Error('poolAI HTTP ' + r.status); }
            return r.json();
        });
}

function resolvePeer() {
    // ?peer= override: plain LAN browsers have no Telegram identity, but
    // both phones share one bound peer — deep-link it and skip lookup.
    var qp = queryPeer();
    if (qp) {
        S.peer = qp;
        saveWorkerState();
        return Promise.resolve(qp);
    }
    var user = tgUser();
    if (!user) {
        return Promise.reject(new Error('open from Telegram to identify'));
    }
    return timed('/api/edge/workers', { headers: { Accept: 'application/json' } }, 15000)
        .then(function (r) { return r.json(); })
        .then(function (data) {
            var rows = (data && data.workers) || [];
            for (var i = 0; i < rows.length; i++) {
                if (String(rows[i].telegram_user_id) === user) {
                    S.peer = rows[i].peer_id;
                    saveWorkerState();
                    return S.peer;
                }
            }
            throw new Error('no bound peer for this account (bind first)');
        });
}

function bootConfig() {
    setStatus('loading tensor config&hellip;');
    return timed('/api/edge/tensor/config', {
        headers: { Accept: 'application/json' }
    }, 15000).then(function (r) { return r.json(); }).then(function (data) {
        applyConfig(data);
    }).catch(function () {
        applyConfig(null);
    });
}

function applyConfig(data) {
    var ok = data && data.runtime && data.runtime.esm && data.runtime.wasm;
    RT = ok
        ? { esm: data.runtime.esm, wasm: data.runtime.wasm }
        : { esm: HF_RT.esm, wasm: HF_RT.wasm };
    MODELS = {};
    DLS = {};
    var ms = (data && data.models) || [];
    for (var i = 0; i < ms.length; i++) {
        if (ms[i] && ms[i].key && ms[i].url) {
            MODELS[ms[i].key] = {
                label: ms[i].label || ms[i].key,
                url: ms[i].url,
                size_mb: ms[i].size_mb || 0
            };
            DLS[ms[i].key] = freshDl();
        }
    }
    if (!Object.keys(MODELS).length) {
        MODELS = HF_MODELS;
        for (var k in HF_MODELS) {
            if (Object.prototype.hasOwnProperty.call(HF_MODELS, k)) { DLS[k] = freshDl(); }
        }
        setStatus('host catalog empty — HuggingFace fallback.');
    } else {
        setStatus('host catalog: ' + Object.keys(MODELS).length + ' model(s), runtime local.');
    }
    renderRows();
    refreshCachedTags();
}

function loadRuntime() {
    // Dynamic import keeps the page alive when the runtime is unreachable;
    // failures surface in status instead of killing the whole script.
    if (S.rt) { return Promise.resolve(S.rt); }
    return import(RT.esm).then(function (mod) {
        if (!mod || !mod.Wllama) { throw new Error('bad runtime module'); }
        S.rt = { Wllama: mod.Wllama, wasmUrl: RT.wasm };
        return S.rt;
    });
}

// ---- per-model download state (torrent-style) ----

function freshDl() {
    return {
        status: 'idle', // idle | active | paused | done | error
        loaded: 0, total: 0, speed: 0,
        chunks: [], ctrl: null,
        t0: 0, lastT: 0, lastLoaded: 0, lastByte: 0,
        auto: false, err: ''
    };
}

function fmtMB(b) { return (b / 1048576).toFixed(1); }

function fmtSpeed(bps) {
    if (!isFinite(bps) || bps <= 0) { return '--'; }
    return bps > 1048576
        ? (bps / 1048576).toFixed(1) + 'MB/s'
        : Math.round(bps / 1024) + 'KB/s';
}

function fnameOf(entry) {
    if (!entry) { return 'model.gguf'; }
    if (entry.url) {
        var m = /\/([^\/\?#]+)(?:[\?#]|$)/.exec(entry.url);
        if (m) { return m[1]; }
    }
    return entry.file || 'model.gguf';
}

function activeKey() {
    var keys = Object.keys(DLS);
    for (var i = 0; i < keys.length; i++) {
        var st = DLS[keys[i]].status;
        if (st === 'active' || st === 'paused') { return keys[i]; }
    }
    return '';
}

function rowEl(key) {
    return el('dl-' + key);
}

function renderRows() {
    // One torrent-style row per model: state text + contextual buttons.
    var box = el('tensor-models');
    if (!box) { return; }
    while (box.firstChild) { box.removeChild(box.firstChild); }
    var keys = Object.keys(MODELS);
    // Largest phone-sane model first (~1.2GB cap keeps 10GB files away
    // from the top); the rest follow catalog order.
    keys.sort(function (a, b) {
        var sa = MODELS[a].size_mb || 0;
        var sb = MODELS[b].size_mb || 0;
        var pa = (!sa || sa <= 1200) ? 0 : 1;
        var pb = (!sb || sb <= 1200) ? 0 : 1;
        if (pa !== pb) { return pa - pb; }
        return sb - sa;
    });
    for (var i = 0; i < keys.length; i++) {
        (function (key) {
            var entry = MODELS[key];
            var card = document.createElement('div');
            card.className = 'gsv-card model-row';
            card.id = 'dl-' + key;
            var title = document.createElement('div');
            var nm = document.createElement('strong');
            nm.textContent = entry.label;
            title.appendChild(nm);
            var tag = document.createElement('span');
            tag.className = 'pill pill-warn';
            tag.id = 'tag-' + key;
            tag.textContent = 'new';
            tag.style.marginLeft = '8px';
            title.appendChild(tag);
            card.appendChild(title);
            var stat = document.createElement('div');
            stat.className = 'stat-line';
            stat.id = 'stat-' + key;
            stat.textContent = 'idle';
            card.appendChild(stat);
            var grid = document.createElement('div');
            grid.className = 'btn-grid';
            var acts = ['download', 'pause', 'cancel', 'use'];
            for (var j = 0; j < acts.length; j++) {
                (function (act) {
                    var b = document.createElement('button');
                    b.type = 'button';
                    b.className = 'tab';
                    b.id = 'btn-' + act + '-' + key;
                    b.textContent = act === 'pause' && DLS[key].status === 'paused'
                        ? 'Resume'
                        : act[0].toUpperCase() + act.slice(1);
                    b.addEventListener('click', function () { rowAction(key, act); });
                    grid.appendChild(b);
                })(acts[j]);
            }
            card.appendChild(grid);
            box.appendChild(card);
        })(keys[i]);
    }
    refreshRows();
    refreshCachedTags();
}

function rowAction(key, act) {
    if (act === 'download') { startDownload(key); }
    else if (act === 'pause') {
        var dl = DLS[key];
        if (dl && dl.status === 'paused') { resumeDownload(key); }
        else { pauseDownload(key); }
    }
    else if (act === 'cancel') { cancelDownload(key); }
    else if (act === 'use') { useModel(key); }
}

function refreshRows() {
    // Button visibility follows state; every control always answers.
    var keys = Object.keys(DLS);
    for (var i = 0; i < keys.length; i++) {
        (function (key) {
            var dl = DLS[key];
            var show = function (id, vis) {
                var b = el('btn-' + id + '-' + key);
                if (b) { b.style.display = vis ? '' : 'none'; }
            };
            var stat = el('stat-' + key);
            var pct = dl.total ? Math.round((dl.loaded / dl.total) * 100) : 0;
            var line = dl.status;
            if (dl.status === 'active' || dl.status === 'paused') {
                line += ' ' + fmtMB(dl.loaded) + '/' + (dl.total ? fmtMB(dl.total) + 'MB' : '?') +
                    ' (' + pct + '%) @ ' + fmtSpeed(dl.speed);
            } else if (dl.status === 'done' || dl.status === 'cached') {
                line += dl.total ? ' ' + fmtMB(dl.total) + 'MB' : '';
            } else if (dl.status === 'error') {
                line += ' ' + dl.err;
            }
            line += ' · 1 seed (host LAN) · peers 1';
            if (stat) { stat.textContent = line; }
            show('download', dl.status === 'idle' || dl.status === 'error');
            var pauseBtn = el('btn-pause-' + key);
            if (pauseBtn) {
                pauseBtn.style.display = (dl.status === 'active' || dl.status === 'paused') ? '' : 'none';
                pauseBtn.textContent = dl.status === 'paused' ? 'Resume' : 'Pause';
            }
            show('cancel', dl.status === 'active' || dl.status === 'paused');
            var useBtn = el('btn-use-' + key);
            if (useBtn) {
                useBtn.style.display = (dl.status === 'done' || dl.status === 'cached') ? '' : 'none';
                if (S.modelKey === key && S.wllama) {
                    useBtn.textContent = 'In use';
                } else {
                    useBtn.textContent = 'Use';
                }
            }
        })(keys[i]);
    }
}

function totalFromHeaders(r) {
    try {
        var cr = r.headers.get('Content-Range') || '';
        var m = /\/(\d+)\s*$/.exec(cr);
        if (m) { return parseInt(m[1], 10) || 0; }
        var cl = r.headers.get('Content-Length');
        return cl ? parseInt(cl, 10) || 0 : 0;
    } catch (e) { return 0; }
}

function startDownload(key) {
    var entry = MODELS[key];
    var dl = DLS[key];
    if (!entry || !dl) { return; }
    if (dl.status === 'active' || dl.status === 'paused') { return; }
    var busy = activeKey();
    if (busy && busy !== key) {
        setStatus('finish or cancel ' + esc(MODELS[busy].label) + ' first');
        return;
    }
    if (dl.status === 'done') {
        useModel(key);
        return;
    }
    if (dl.status === 'cached') {
        // Bytes already on device: hydrate RAM and mark done, no fetch.
        setStatus('reading ' + esc(entry.label) + ' from device cache&hellip;');
        IDB.get(fnameOf(entry)).then(function (rec) {
            if (rec && rec.blob) {
                BYTESTORE.set(fnameOf(entry), { blob: rec.blob, size: rec.blob.size });
                dl.status = 'done';
                dl.total = rec.blob.size;
                dl.loaded = rec.blob.size;
                refreshRows();
                logRow('sys', 'hydrated from device cache (' + fmtMB(rec.blob.size) + 'MB)');
            } else {
                dl.status = 'idle';
                refreshRows();
                startDownload(key);
            }
        });
        return;
    }
    dl.status = 'active';
    dl.chunks = [];
    dl.loaded = 0;
    dl.total = 0;
    dl.speed = 0;
    dl.err = '';
    dl.t0 = Date.now();
    dl.lastT = dl.t0;
    dl.lastLoaded = 0;
    dl.lastByte = Date.now();
    dl.auto = false;
    wakeLock(true);
    refreshRows();
    dlSegment(key);
}

function pauseDownload(key) {
    var dl = DLS[key];
    if (!dl || dl.status !== 'active' || !dl.ctrl) {
        setStatus('nothing downloading');
        return;
    }
    dl.status = 'paused';
    try { dl.ctrl.abort(); } catch (e) {}
    refreshRows();
    setStatus('paused ' + esc(MODELS[key].label) + ' at ' + fmtMB(dl.loaded) + 'MB');
}

function resumeDownload(key) {
    var dl = DLS[key];
    if (!dl || dl.status !== 'paused') { return; }
    dl.status = 'active';
    dl.lastByte = Date.now();
    refreshRows();
    dlSegment(key);
}

function cancelDownload(key) {
    var dl = DLS[key];
    if (!dl || (dl.status !== 'active' && dl.status !== 'paused')) {
        setStatus('nothing downloading');
        return;
    }
    dl.status = 'idle';
    try { if (dl.ctrl) { dl.ctrl.abort(); } } catch (e) {}
    dl.ctrl = null;
    dl.chunks = [];
    dl.loaded = 0;
    dl.total = 0;
    dl.speed = 0;
    refreshRows();
    var stat = el('stat-' + key);
    if (stat) { stat.textContent = 'cancelled'; }
    setStatus('download cancelled, progress discarded');
    logRow('sys', 'cancelled ' + MODELS[key].label);
    if (!S.running) { wakeLock(false); }
}

function finishDl(key) {
    var dl = DLS[key];
    var entry = MODELS[key];
    var blob = new Blob(dl.chunks, { type: 'application/octet-stream' });
    var name = fnameOf(entry);
    BYTESTORE.set(name, { blob: blob, size: blob.size });
    dl.chunks = [];
    dl.ctrl = null;
    dl.total = blob.size;
    dl.loaded = blob.size;
    refreshRows();
    setStatus('saving ' + esc(entry.label) + ' to device cache&hellip;');
    IDB.put(name, blob).then(function (saved) {
        if (!saved) {
            dl.status = 'done';
            refreshRows();
            logRow('sys', 'device cache unavailable (quota?) — RAM only this session');
            if (!S.running) { wakeLock(false); }
            return;
        }
        IDB.get(name).then(function (rec) {
            var ok = rec && rec.blob && rec.blob.size === blob.size;
            dl.status = 'done';
            refreshRows();
            logRow('sys', ok
                ? 'saved ' + entry.label + ' (' + fmtMB(blob.size) + 'MB, verified)'
                : 'device cache verify FAILED for ' + entry.label);
            if (!S.running) { wakeLock(false); }
        });
    });
}

function dlSegment(key) {
    var dl = DLS[key];
    var entry = MODELS[key];
    if (!dl || !entry) { return; }
    var url = entry.hf
        ? 'https://huggingface.co/' + entry.repo + '/resolve/main/' + entry.file + '?download=true'
        : entry.url;
    dl.ctrl = new AbortController();
    var headers = dl.loaded > 0 ? { Range: 'bytes=' + dl.loaded + '-' } : {};
    fetch(url, { signal: dl.ctrl.signal, headers: headers }).then(function (r) {
        if (r.status !== 200 && r.status !== 206) { throw new Error('HTTP ' + r.status); }
        if (!dl.total) { dl.total = totalFromHeaders(r); }
        var reader = r.body.getReader();
        function pump() {
            return reader.read().then(function (res) {
                if (res.done) { finishDl(key); return; }
                dl.chunks.push(res.value);
                dl.loaded += res.value.byteLength;
                dl.lastByte = Date.now();
                var now = Date.now();
                var dt = (now - dl.lastT) / 1000;
                if (dt >= 0.5) {
                    dl.speed = (dl.loaded - dl.lastLoaded) / dt;
                    dl.lastT = now;
                    dl.lastLoaded = dl.loaded;
                }
                refreshRows();
                return pump();
            });
        }
        return pump();
    }).catch(function (e) {
        // Pause aborts on purpose; auto-resume re-arms itself. Cancelled
        // downloads already reset state — stay silent.
        if (dl.status === 'paused' || dl.auto) { dl.auto = false; refreshRows(); return; }
        if (dl.status !== 'active') { return; }
        dl.status = 'error';
        dl.err = String((e && e.message) || e).slice(0, 120);
        dl.ctrl = null;
        wakeLock(false);
        refreshRows();
        logRow('err', 'download failed: ' + dl.err);
    });
    // Stall watchdog: locked/sleeping radios freeze the stream without
    // aborting it. No bytes for STALL_MS → cut and resume via Range.
    if (!dl.watch) {
        dl.watch = setInterval(function () {
            try {
                if (!dl.ctrl || dl.status !== 'active') { return; }
                if (dl.total && dl.loaded >= dl.total) { return; }
                if (Date.now() - dl.lastByte > STALL_MS) {
                    logRow('sys', 'stall detected — resuming from ' + fmtMB(dl.loaded) + 'MB');
                    dl.auto = true;
                    dl.lastByte = Date.now();
                    try { dl.ctrl.abort(); } catch (e) {}
                    dlSegment(key);
                }
            } catch (e) {}
        }, 5000);
    }
}

// ---- inference (wllama) ----

function loadRuntime() {
    // Dynamic import keeps the page alive when the runtime is unreachable;
    // failures surface in status instead of killing the whole script.
    if (S.rt) { return Promise.resolve(S.rt); }
    return import(RT.esm).then(function (mod) {
        if (!mod || !mod.Wllama) { throw new Error('bad runtime module'); }
        S.rt = { Wllama: mod.Wllama, wasmUrl: RT.wasm };
        return S.rt;
    });
}

function useModel(key) {
    var entry = MODELS[key];
    var dl = DLS[key];
    if (!entry) { return; }
    if (S.modelKey === key && S.wllama) {
        setStatus('already using ' + esc(entry.label));
        return;
    }
    setStatus('preparing ' + esc(entry.label) + '&hellip;');
    bytesFor(key).then(function (blob) {
        if (!blob) {
            setStatus('no bytes — Download ' + esc(entry.label) + ' first');
            return;
        }
        loadIntoWllama(key, entry, blob);
    });
}

function bytesFor(key) {
    var entry = MODELS[key];
    var name = fnameOf(entry);
    var hit = BYTESTORE.get(name);
    if (hit && hit.blob) { return Promise.resolve(hit.blob); }
    return IDB.get(name).then(function (rec) {
        if (rec && rec.blob) {
            BYTESTORE.set(name, { blob: rec.blob, size: rec.blob.size });
            return rec.blob;
        }
        return null;
    });
}

function loadIntoWllama(key, entry, blob) {
    loadRuntime().then(function (rt) {
        var fname = fnameOf(entry);
        var file = null;
        try {
            file = new File([blob], fname, { type: 'application/octet-stream' });
        } catch (e) {
            setStatus('this browser cannot build model files: ' + esc(String((e && e.message) || e)));
            return;
        }
        setStatus('loading ' + esc(entry.label) + ' into engine&hellip;');
        var shim = makeShim();
        shim.seed(fname, blob);
        var inst = null;
        try {
            inst = new rt.Wllama({ default: rt.wasmUrl }, { cacheManager: shim });
        } catch (e) {
            setStatus('engine init failed: ' + esc(String((e && e.message) || e)));
            return;
        }
        inst.loadModel([file], { n_gpu_layers: N_GPU_LAYERS }).then(function () {
            S.wllama = inst;
            S.modelKey = key;
            S.modelLabel = entry.label;
            S.gpu = true;
            saveWorkerState();
            setStatus('model ready: ' + esc(entry.label) + ' (WebGPU)');
            logRow('sys', 'model loaded: ' + entry.label + ' (WebGPU)');
            refreshRows();
        }).catch(function (e) {
            setStatus('WebGPU load failed (' + esc((e && e.message) || e) + '), retrying CPU&hellip;');
            var cpu = null;
            try {
                cpu = new rt.Wllama({ default: rt.wasmUrl }, { cacheManager: makeShim() });
            } catch (e2) {
                setStatus('engine init failed: ' + esc(String((e2 && e2.message) || e2)));
                return;
            }
            cpu.loadModel([file], { n_gpu_layers: 0 }).then(function () {
                S.wllama = cpu;
                S.modelKey = key;
                S.modelLabel = entry.label;
                S.gpu = false;
                saveWorkerState();
                setStatus('model ready: ' + esc(entry.label) + ' (CPU fallback)');
                logRow('sys', 'model loaded: ' + entry.label + ' (CPU fallback)');
                refreshRows();
            }).catch(function (e3) {
                setStatus('model load failed: ' + esc((e3 && e3.message) || e3));
                logRow('err', 'load failed: ' + String((e3 && e3.message) || e3));
            });
        });
    }).catch(function (e) {
        setStatus('runtime load failed: ' + esc(String((e && e.message) || e)));
        logRow('err', 'runtime import failed: ' + String((e && e.message) || e));
    });
}

// Minimal cache object: the constructor demands OPFS (absent on HTTP LAN
// + old WebViews), so a stub keeps init alive; files load directly.
function makeShim() {
    var mem = new Map();
    return {
        download: function () { return Promise.reject(new Error('direct files only')); },
        list: function () {
            var out = [];
            mem.forEach(function (v, k) { out.push({ name: k, size: v.size }); });
            BYTESTORE.forEach(function (v, k) {
                if (!mem.has(k)) { out.push({ name: k, size: v.size }); }
            });
            return Promise.resolve(out);
        },
        open: function (name) {
            var e = mem.get(name) || BYTESTORE.get(name);
            return Promise.resolve(e ? e.blob : null);
        },
        seed: function (name, blob) { mem.set(name, { blob: blob, size: blob.size }); }
    };
}

function countTokens(text) {
    if (!S.wllama || !S.wllama.tokenize) { return Math.ceil(String(text).length / 4); }
    try {
        var t = S.wllama.tokenize(String(text));
        if (t && typeof t.then === 'function') { return -1; } // async variant: skip
        return (t && t.length) || Math.ceil(String(text).length / 4);
    } catch (e) { return Math.ceil(String(text).length / 4); }
}

function runChat(prompt, maxTokens) {
    var t0 = (typeof performance !== 'undefined' && performance.now) ? performance.now() : Date.now();
    return S.wllama.createChatCompletion({
        messages: [{ role: 'user', content: String(prompt) }],
        max_tokens: maxTokens > 0 ? maxTokens : 64,
        temperature: 0
    }).then(function (resp) {
        var t1 = (typeof performance !== 'undefined' && performance.now) ? performance.now() : Date.now();
        var text = '';
        try { text = resp.choices[0].message.content || ''; } catch (e) {}
        var toks = countTokens(text);
        var secs = Math.max((t1 - t0) / 1000, 0.001);
        var tps = toks > 0 ? (toks / secs).toFixed(2) : '?';
        return { text: text, ms: Math.round(t1 - t0), tps: tps };
    });
}

function completeTask(taskId, answer) {
    return pool('/virtual-nodes/' + encodeURIComponent(S.peer) + '/tasks/' +
        encodeURIComponent(taskId) + '/complete', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ status: 'done', detail: answer })
        }, 20000);
}

function requeue(task) {
    return pool('/virtual-nodes/' + encodeURIComponent(S.peer) + '/tasks', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ task_type: task.task_type, payload: task.payload || {} })
    }, 20000);
}

function serveTask(task) {
    var payload = task.payload || {};
    var prompt = payload.prompt || '';
    var maxTokens = parseInt(payload.max_tokens, 10) || 64;
    if (!prompt) {
        logRow('skip', 'llama_chat without prompt (id ' + task.id + ')');
        return completeTask(task.id, '(empty prompt)').then(function () {});
    }
    logRow('task', 'llama_chat <- ' + prompt.slice(0, 120));
    return runChat(prompt, maxTokens).then(function (r) {
        logRow('done', r.tps + ' tok/s, ' + r.ms + 'ms :: ' + r.text.slice(0, 200));
        return completeTask(task.id, r.text).then(function () {
            S.done++;
            saveWorkerState();
            setStatus('worker up: ' + S.done + ' tasks done (' + esc(S.modelLabel) +
                (S.gpu ? ', WebGPU' : ', CPU') + ')');
        });
    }).catch(function (e) {
        logRow('err', 'task failed: ' + String((e && e.message) || e));
        return completeTask(task.id, '(worker error: ' + String((e && e.message) || e) + ')')
            .then(function () {});
    });
}

function pollOnce() {
    if (S.busy) { return Promise.resolve(); }
    S.busy = true;
    var requeued = 0;
    function next() {
        return pool('/virtual-nodes/' + encodeURIComponent(S.peer) + '/tasks/poll', {
            headers: { Accept: 'application/json' }
        }, 20000).then(function (data) {
            var task = data && data.task;
            if (!task) { return 'idle'; }
            if (task.task_type === 'llama_chat') { return serveTask(task); }
            // Not ours (bootstrap/pc tasks): put back, keep looking briefly.
            if (requeued < MAX_REQUEUE_PER_TICK) {
                requeued++;
                return requeue(task).then(next);
            }
            return 'deferred';
        });
    }
    return next().catch(function (e) {
        logRow('err', 'poll: ' + String((e && e.message) || e));
    }).then(function () { S.busy = false; });
}

function startLoop() {
    if (!S.wllama) {
        setStatus('tap Use on a downloaded model first');
        return;
    }
    if (!S.peer) {
        setStatus('resolving peer&hellip;');
        resolvePeer().then(function (peer) {
            setStatus('worker up on ' + esc(peer) + ', polling every ' + (POLL_MS / 1000) + 's');
            logRow('sys', 'bound peer: ' + peer);
            beginLoop();
        }).catch(function (e) {
            setStatus('peer: ' + esc((e && e.message) || e));
        });
        return;
    }
    beginLoop();
}

function beginLoop() {
    if (S.running) { setStatus('already running'); return; }
    S.running = true;
    S.wpaused = false;
    wakeLock(true);
    setStatus('worker up on ' + esc(S.peer));
    el('tensor-start').disabled = true;
    el('tensor-stop').disabled = false;
    el('tensor-wpause').disabled = false;
    el('tensor-wpause').textContent = 'Pause worker';
    var tick = function () {
        if (!S.running) { return; }
        if (S.wpaused) {
            S.timer = setTimeout(tick, POLL_MS);
            return;
        }
        pollOnce().then(function () {
            if (S.running) { S.timer = setTimeout(tick, POLL_MS); }
        });
    };
    tick();
}

function pauseWorker() {
    if (!S.running) { setStatus('worker not running'); return; }
    if (!S.wpaused) {
        S.wpaused = true;
        el('tensor-wpause').textContent = 'Resume worker';
        setStatus('worker paused on ' + esc(S.peer) + ' (queue untouched)');
        logRow('sys', 'worker paused');
    } else {
        S.wpaused = false;
        el('tensor-wpause').textContent = 'Pause worker';
        setStatus('worker resumed');
        logRow('sys', 'worker resumed');
    }
}

function stopLoop() {
    S.running = false;
    S.wpaused = false;
    wakeLock(false);
    if (S.timer) { clearTimeout(S.timer); S.timer = null; }
    el('tensor-start').disabled = false;
    el('tensor-stop').disabled = true;
    el('tensor-wpause').disabled = true;
    el('tensor-wpause').textContent = 'Pause worker';
    setStatus('worker stopped (' + S.done + ' tasks done)');
}

function selfTest() {
    if (!S.wllama) { setStatus('tap Use on a downloaded model first'); return; }
    var prompt = el('tensor-prompt').value.trim();
    if (!prompt) { setStatus('type a prompt first'); return; }
    setStatus('self-test running&hellip;');
    runChat(prompt, 64).then(function (r) {
        setStatus('self-test: ' + r.tps + ' tok/s, ' + r.ms + 'ms');
        logRow('self', r.tps + ' tok/s :: ' + r.text.slice(0, 200));
    }).catch(function (e) {
        setStatus('self-test failed: ' + esc((e && e.message) || e));
    });
}

function saveWorkerState() {
    try {
        localStorage.setItem('tensor-worker', JSON.stringify({
            modelKey: S.modelKey,
            modelLabel: S.modelLabel,
            peer: S.peer,
            done: S.done
        }));
    } catch (e) {}
}

function restoreWorkerState() {
    try {
        var raw = localStorage.getItem('tensor-worker');
        if (!raw) { return; }
        var st = JSON.parse(raw);
        if (st && (st.modelKey || st.peer)) {
            setStatus('last session: ' + esc(st.modelLabel || st.modelKey || '?') +
                ', peer ' + esc(st.peer || '?') + ', ' + (st.done || 0) + ' tasks done.' +
                ' Model bytes persist on-device — Use the model, then Start.');
        }
    } catch (e) {}
}

el('tensor-start').addEventListener('click', startLoop);
el('tensor-stop').addEventListener('click', stopLoop);
el('tensor-stop').disabled = true;
el('tensor-wpause').addEventListener('click', pauseWorker);
el('tensor-wpause').disabled = true;
el('tensor-self').addEventListener('click', selfTest);
window.__tensorReady = true;
// Catalog first (host vendor + GGUF library), so the list reflects what
// this box actually serves; HF fallback when the host has no catalog.
bootConfig();
lanHint();
restoreWorkerState();
IDB.open().then(function (ok) {
    if (ok) {
        logRow('sys', 'device cache ready (IndexedDB)');
        try {
            if (navigator.storage && navigator.storage.persist) {
                navigator.storage.persist();
            }
        } catch (e) {}
        refreshCachedTags();
    } else {
        logRow('sys', 'device cache unavailable (no IndexedDB) — RAM only');
    }
});

function refreshCachedTags() {
    if (!IDB.ok) { return; }
    var box = el('tensor-models');
    if (!box || !box.children.length) { return; }
    IDB.listAll().then(function (rows) {
        var cached = {};
        for (var i = 0; i < rows.length; i++) {
            cached[rows[i].name] = rows[i].size;
        }
        for (var j = 0; j < box.children.length; j++) {
            (function (card) {
                var key = card.getAttribute('data-key');
                var entry = MODELS[key];
                var fname = entry && entry.url
                    ? entry.url.split('/').pop()
                    : (entry && entry.file) || '';
                var tag = card.querySelector('.cache-tag');
                if (fname && cached[fname] > 0) {
                    if (tag) {
                        tag.textContent = '[cached]';
                    }
                    var dl = DLS[key];
                    if (dl && dl.status === 'idle') {
                        dl.status = 'cached';
                        refreshRows();
                    }
                }
            })(box.children[j]);
        }
        if (rows.length) {
            logRow('sys', 'on device: ' + rows.length + ' model file(s)');
        } else {
            logRow('sys', 'device cache empty — first Load downloads');
        }
    });
}
