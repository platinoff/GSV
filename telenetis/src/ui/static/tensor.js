/* Browser tensor worker (swarm path b, PoC ticket t-1789571785502387700).
 *
 * Runs a GGUF model IN THIS PHONE via wllama (llama.cpp WASM + WebGPU,
 * fastest current browser runtime per arXiv:2605.20706: +54% decode vs
 * WebLLM, GGUF-native so our Qwen quants load unconverted), then serves
 * `llama_chat` tasks from this device's poolAI peer queue:
 *   poll GET /edge/upstream/poolai/api/v1/virtual-nodes/{peer}/tasks/poll
 *   run locally -> POST .../tasks/{id}/complete {status, detail}
 * Same origin through the Telenetis reverse proxy, so no CORS and no
 * bearer handling in the browser. Non-chat tasks are re-queued untouched
 * (poll pops — dropping them would lose PC-side work).
 *
 * Constraints: needs WebGPU (else CPU fallback via n_gpu_layers 0);
 * needs the model download once (OPFS-cached after); PC pollers race for
 * the same queue during the PoC (steady-state routing is a follow-up).
 *
 * Classic script (no static imports): the wllama CDN modules load lazily
 * via dynamic import() on Load, so a dead CDN leaves the model list and
 * buttons alive and surfaces the exact error instead of a dead page.
 */
var WLLAMA_PIN = '3.5.1';
var N_GPU_LAYERS = 99; // all layers to WebGPU; CPU fallback on failure
var POLL_MS = 5000;
var MAX_REQUEUE_PER_TICK = 3;

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
    done: 0
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
    var ms = (data && data.models) || [];
    for (var i = 0; i < ms.length; i++) {
        if (ms[i] && ms[i].key && ms[i].url) {
            MODELS[ms[i].key] = {
                label: ms[i].label || ms[i].key,
                url: ms[i].url,
                size_mb: ms[i].size_mb || 0
            };
        }
    }
    if (!Object.keys(MODELS).length) {
        MODELS = HF_MODELS;
        setStatus('host catalog empty — HuggingFace fallback. 1) Load model 2) Start worker.');
    } else {
        setStatus('host catalog: ' + Object.keys(MODELS).length +
            ' model(s), runtime local. 1) Load model 2) Start worker.');
    }
    fillModels();
    tagCached();
}

function loadWllama() {
    // Dynamic import keeps the page alive when the runtime is unreachable;
    // failures surface in status instead of killing the whole script.
    return import(RT.esm).then(function (mod) {
        if (!mod || !mod.Wllama) { throw new Error('bad runtime module'); }
        return { Wllama: mod.Wllama, wasmUrl: RT.wasm };
    });
}

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
        // Every cached model with its byte size (drives the "(cached)"
        // tags, so an empty device is visible instead of silent).
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

function startModelLoad(rt, spec, key) {
    // Custom RAM cache: wllama's default CacheManager demands OPFS
    // (absent on HTTP LAN + old WebViews → "No supported storage backend").
    // The shim speaks the 3 methods ModelManager uses (download/list/open),
    // fetches with pause/resume + speed stats, keeps bytes in RAM, and
    // persists finished files to IndexedDB (works insecure + old) so
    // reloads reuse them instead of re-downloading.
    var store = new Map();
    function fname(url) {
        var m = /\/([^\/\?#]+)(?:[\?#]|$)/.exec(url || '');
        return m ? m[1] : String(url);
    }
    var SEG = {
        chunks: [], loaded: 0, total: 0, ctrl: null, t0: 0,
        lastT: 0, lastLoaded: 0, speed: 0, paused: false, url: '',
        resolve: null, reject: null, onP: null, name: '',
        auto: false, lastByte: 0
    };
    function fmtMB(b) { return (b / 1048576).toFixed(1); }
    function fmtSpeed(bps) {
        if (!isFinite(bps) || bps <= 0) { return '--'; }
        return bps > 1048576
            ? (bps / 1048576).toFixed(1) + 'MB/s'
            : Math.round(bps / 1024) + 'KB/s';
    }
    function reportDl() {
        var pct = SEG.total ? Math.round((SEG.loaded / SEG.total) * 100) : 0;
        var box = el('tensor-torrent');
        if (box) {
            box.textContent =
                '\u25BC ' + fmtSpeed(SEG.speed) + ' \u00B7 ' +
                fmtMB(SEG.loaded) + '/' + (SEG.total ? fmtMB(SEG.total) : '?') + 'MB' +
                ' (' + pct + '%) \u00B7 1 seed (host LAN) \u00B7 peers 1' +
                (SEG.paused ? ' \u00B7 paused' : '');
        }
        try {
            if (SEG.onP) { SEG.onP({ loaded: SEG.loaded, total: SEG.total }); }
        } catch (e) {}
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
    function tickSpeed() {
        var now = Date.now();
        var dt = (now - SEG.lastT) / 1000;
        if (dt >= 0.5) {
            SEG.speed = (SEG.loaded - SEG.lastLoaded) / dt;
            SEG.lastT = now;
            SEG.lastLoaded = SEG.loaded;
        }
    }
    function finishDl() {
        var blob = new Blob(SEG.chunks, { type: 'application/octet-stream' });
        store.set(SEG.name, { blob: blob, size: blob.size });
        SEG.ctrl = null;
        reportDl();
        setStatus('saving to device cache&hellip;');
        IDB.put(SEG.name, blob).then(function (saved) {
            if (!saved) {
                logRow('sys', 'device cache unavailable (quota?) — RAM only this session');
                SEG.resolve({ name: SEG.name, size: blob.size });
                if (!S.running) { wakeLock(false); }
                return;
            }
            // Verify-after-write: read back and compare sizes, so a
            // half-written entry can never look cached.
            IDB.get(SEG.name).then(function (rec) {
                var ok = rec && rec.blob && rec.blob.size === blob.size;
                logRow('sys', ok
                    ? 'saved to device cache (' + fmtMB(blob.size) + 'MB, verified)'
                    : 'device cache verify FAILED — will re-download next time');
                SEG.resolve({ name: SEG.name, size: blob.size });
                if (!S.running) { wakeLock(false); }
            });
        });
    }
    function dlSegment() {
        var from = SEG.loaded;
        SEG.ctrl = new AbortController();
        var headers = from > 0 ? { Range: 'bytes=' + from + '-' } : {};
        fetch(SEG.url, { signal: SEG.ctrl.signal, headers: headers }).then(function (r) {
            if (r.status !== 200 && r.status !== 206) { throw new Error('HTTP ' + r.status); }
            if (!SEG.total) { SEG.total = totalFromHeaders(r); }
            var reader = r.body.getReader();
            function pump() {
                return reader.read().then(function (res) {
                    if (res.done) { finishDl(); return; }
                    SEG.chunks.push(res.value);
                    SEG.loaded += res.value.byteLength;
                    SEG.lastByte = Date.now();
                    tickSpeed();
                    reportDl();
                    return pump();
                });
            }
            return pump();
        }).catch(function (e) {
            // Pause aborts the segment on purpose: hold chunks, wait resume.
            // Auto-resume (stall watchdog) re-arms itself the same way.
            // Cancelled downloads null resolve/reject first: stay silent.
            if (SEG.paused || SEG.auto) { SEG.auto = false; reportDl(); return; }
            SEG.ctrl = null;
            wakeLock(false);
            if (SEG.reject) { SEG.reject(e); }
        });
    }
    // Stall watchdog: locked/sleeping radios freeze the stream without
    // aborting it. No bytes for 20s → cut the dead segment and resume
    // from SEG.loaded via Range (once per watchdog pass at most).
    if (!window.__tensorStallWatch) {
        window.__tensorStallWatch = setInterval(function () {
            try {
                if (!SEG.ctrl || SEG.paused || SEG.lastByte === 0) { return; }
                if (SEG.total && SEG.loaded >= SEG.total) { return; }
                if (Date.now() - SEG.lastByte > 20000) {
                    logRow('sys', 'stall detected — resuming from ' +
                        (SEG.loaded / 1048576).toFixed(1) + 'MB');
                    SEG.auto = true;
                    SEG.lastByte = Date.now();
                    try { SEG.ctrl.abort(); } catch (e) {}
                    dlSegment();
                }
            } catch (e) {}
        }, 5000);
    }
    var shim = {
        download: function (url, opts) {
            SEG.url = url;
            SEG.name = fname(url);
            SEG.chunks = [];
            SEG.loaded = 0;
            SEG.total = 0;
            SEG.paused = false;
            SEG.t0 = Date.now();
            SEG.lastT = SEG.t0;
            SEG.lastLoaded = 0;
            SEG.speed = 0;
            SEG.onP = (opts && opts.progressCallback) || null;
            setPausedUI(false);
            SEG.lastByte = Date.now();
            wakeLock(true);
            return new Promise(function (resolve, reject) {
                SEG.resolve = resolve;
                SEG.reject = reject;
                // Device cache first: reloads skip the download entirely.
                IDB.get(SEG.name).then(function (rec) {
                    if (rec && rec.blob && rec.blob.size > 0) {
                        store.set(SEG.name, { blob: rec.blob, size: rec.blob.size });
                        SEG.loaded = rec.blob.size;
                        SEG.total = rec.blob.size;
                        reportDl();
                        logRow('sys', 'loaded from device cache (' +
                            fmtMB(rec.blob.size) + 'MB) — no download');
                        SEG.resolve({ name: SEG.name, size: rec.blob.size });
                        return;
                    }
                    dlSegment();
                });
            });
        },
        list: function () {
            var out = [];
            store.forEach(function (v, k) { out.push({ name: k, size: v.size }); });
            return Promise.resolve(out);
        },
        open: function (name) {
            var e = store.get(name);
            if (e) { return Promise.resolve(e.blob); }
            // RAM missed (fresh reload): fall back to the device cache.
            return IDB.get(name).then(function (rec) {
                if (rec && rec.blob) {
                    store.set(name, { blob: rec.blob, size: rec.blob.size });
                    return rec.blob;
                }
                return null;
            });
        }
    };
    window.__tensorPause = function () {
        if ((!SEG.ctrl && SEG.loaded === 0) || SEG.paused) {
            if (!SEG.paused) { setStatus('nothing downloading'); }
            return;
        }
        if (!SEG.ctrl || SEG.paused) { return; }
        SEG.paused = true;
        try { SEG.ctrl.abort(); } catch (e) {}
        setPausedUI(true);
        reportDl();
    };
    window.__tensorResume = function () {
        if (!SEG.paused) { return; }
        SEG.paused = false;
        setPausedUI(false);
        reportDl();
        dlSegment();
    };
    var inst = new rt.Wllama({ default: rt.wasmUrl }, { cacheManager: shim });
    var onProgress = function (loaded, total) {
        var pct = total ? Math.round((loaded / total) * 100) : 0;
        setStatus('downloading ' + esc(spec.label) + ': ' + pct + '%');
    };
    var opts = { progressCallback: onProgress, n_gpu_layers: N_GPU_LAYERS };
    // Same-origin host URL (local vendor + LAN model bytes) or the
    // HuggingFace pair when the host serves no catalog.
    var load = spec.hf
        ? inst.loadModelFromHF({ repo: spec.repo, file: spec.file }, opts)
        : inst.loadModelFromUrl(spec.url, opts);
    return load.then(function () {
        S.wllama = inst;
        S.modelKey = key;
        S.modelLabel = spec.label;
        S.gpu = true;
        saveWorkerState();
        setStatus('model ready: ' + esc(spec.label) + ' (WebGPU)');
        logRow('sys', 'model loaded: ' + spec.label + ' (WebGPU)');
    }).catch(function (e) {
        // Redmi-class devices may fail the GPU path: retry CPU-only.
        setStatus('WebGPU load failed (' + esc((e && e.message) || e) + '), retrying CPU&hellip;');
        var cpu = new rt.Wllama({ default: rt.wasmUrl });
        var cpuOpts = { progressCallback: onProgress, n_gpu_layers: 0 };
        var cpuLoad = spec.hf
            ? cpu.loadModelFromHF({ repo: spec.repo, file: spec.file }, cpuOpts)
            : cpu.loadModelFromUrl(spec.url, cpuOpts);
        return cpuLoad.then(function () {
            S.wllama = cpu;
            S.modelKey = key;
            S.modelLabel = spec.label;
            S.gpu = false;
            saveWorkerState();
            setStatus('model ready: ' + esc(spec.label) + ' (CPU fallback)');
            logRow('sys', 'model loaded: ' + spec.label + ' (CPU fallback)');
        });
    }).catch(function (e2) {
        setStatus('model load failed: ' + esc((e2 && e2.message) || e2));
        logRow('err', 'load failed: ' + String((e2 && e2.message) || e2));
    });
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
    // No model yet: load it first, then continue into the loop instead of
    // bouncing the user back to the Load button.
    if (!S.wllama) {
        setStatus('loading model first&hellip;');
        loadModelAsync().then(function (ready) {
            if (ready) { startLoop(); }
        });
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

// Promise version of the Load button flow (true when the model is ready).
// Button handler keeps fire-and-forget behavior via the same path.
function loadModelAsync() {
    var key = el('tensor-model').value || Object.keys(MODELS)[0] || 'qwen15';
    var spec = MODELS[key] || MODELS.qwen15;
    if (!spec) { setStatus('no models available'); return Promise.resolve(false); }
    setStatus('loading runtime&hellip;');
    return loadWllama().then(function (rt) {
        setStatus('loading ' + esc(spec.label) + '&hellip;');
        return startModelLoad(rt, spec, key);
    }).then(function () {
        return !!S.wllama;
    }).catch(function (e) {
        setStatus('runtime load failed: ' + esc(String((e && e.message) || e)));
        logRow('err', 'runtime import failed: ' + String((e && e.message) || e));
        wakeLock(false);
        return false;
    });
}

function loadModel() { loadModelAsync(); }

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

function cancelDownload() {
    if (!SEG.ctrl && SEG.loaded === 0 && !SEG.paused) {
        setStatus('nothing downloading');
        return;
    }
    SEG.paused = false;
    try { if (SEG.ctrl) { SEG.ctrl.abort(); } catch (e) {}
    SEG.ctrl = null;
    SEG.chunks = [];
    SEG.loaded = 0;
    SEG.total = 0;
    SEG.speed = 0;
    SEG.resolve = null;
    SEG.reject = null;
    setPausedUI(false);
    var box = el('tensor-torrent');
    if (box) { box.textContent = 'download cancelled'; }
    setStatus('download cancelled, progress discarded');
    logRow('sys', 'download cancelled');
    if (!S.running) { wakeLock(false); }
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
    if (!S.wllama) { setStatus('load the model first'); return; }
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

function setPausedUI(paused) {
    var b = el('tensor-pause');
    if (!b) { return; }
    b.textContent = paused ? 'Resume' : 'Pause';
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
                ' Model bytes persist on-device — Load reuses them, then Start.');
        }
    } catch (e) {}
}

function fillModels() {
    var sel = el('tensor-model');
    sel.innerHTML = '';
    var keys = Object.keys(MODELS);
    // Phone-sane default: largest model within ~1.2GB (never auto-pick a
    // 10GB dense/MoE file on a phone); the owner can still choose manually.
    var best = keys[0] || '';
    var bestSize = -1;
    keys.forEach(function (k) {
        var o = document.createElement('option');
        o.value = k;
        o.textContent = MODELS[k].label;
        sel.appendChild(o);
        var sz = MODELS[k].size_mb || 0;
        if ((sz <= 0 || sz <= 1200) && sz > bestSize) {
            bestSize = sz;
            best = k;
        }
    });
    if (best) { sel.value = best; }
}

el('tensor-load').addEventListener('click', loadModel);
el('tensor-start').addEventListener('click', startLoop);
el('tensor-stop').addEventListener('click', stopLoop);
el('tensor-stop').disabled = true;
el('tensor-wpause').addEventListener('click', pauseWorker);
el('tensor-wpause').disabled = true;
el('tensor-cancel').addEventListener('click', cancelDownload);
el('tensor-self').addEventListener('click', selfTest);
el('tensor-pause').addEventListener('click', function () {
    try {
        var label = el('tensor-pause').textContent || '';
        if (label === 'Resume') {
            if (window.__tensorResume) { window.__tensorResume(); }
        } else {
            if (window.__tensorPause) { window.__tensorPause(); }
        }
    } catch (e) {}
});
window.__tensorReady = true;
// Catalog first (host vendor + GGUF library), so the list reflects what
// this box actually serves; HF fallback when the host has no catalog.
bootConfig();
lanHint();
restoreWorkerState();
// Storage gate: Load waits for IndexedDB (avoids racing an unopened
// store and re-downloading). Cached models get [cached] tags from both
// sides (catalog fetch and IDB open race each other; tagging is
// idempotent), so an empty device is visible instead of silent.
el('tensor-load').disabled = true;
function tagCached() {
    if (!IDB.ok) { return; }
    var sel = el('tensor-model');
    if (!sel || !sel.options.length) { return; }
    IDB.listAll().then(function (rows) {
        var cached = {};
        for (var i = 0; i < rows.length; i++) {
            cached[rows[i].name] = rows[i].size;
        }
        for (var j = 0; j < sel.options.length; j++) {
            var opt = sel.options[j];
            var entry = MODELS[opt.value];
            var fname = entry && entry.url
                ? entry.url.split('/').pop()
                : (entry && entry.file) || '';
            if (fname && cached[fname] > 0 && opt.textContent.indexOf('[cached]') < 0) {
                opt.textContent = entry.label + ' [cached]';
            }
        }
        if (rows.length) {
            logRow('sys', 'on device: ' + rows.length + ' model file(s)');
        } else {
            logRow('sys', 'device cache empty — first Load downloads');
        }
    });
}
IDB.open().then(function (ok) {
    el('tensor-load').disabled = false;
    if (ok) {
        logRow('sys', 'device cache ready (IndexedDB)');
        try {
            if (navigator.storage && navigator.storage.persist) {
                navigator.storage.persist();
            }
        } catch (e) {}
        tagCached();
    } else {
        logRow('sys', 'device cache unavailable (no IndexedDB) — RAM only');
    }
});
