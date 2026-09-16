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

function tgUser() {
    try {
        var w = window.Telegram && window.Telegram.WebApp;
        if (w && w.initDataUnsafe && w.initDataUnsafe.user && w.initDataUnsafe.user.id) {
            return String(w.initDataUnsafe.user.id);
        }
    } catch (e) {}
    return '';
}

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
}

function loadWllama() {
    // Dynamic import keeps the page alive when the runtime is unreachable;
    // failures surface in status instead of killing the whole script.
    return import(RT.esm).then(function (mod) {
        if (!mod || !mod.Wllama) { throw new Error('bad runtime module'); }
        return { Wllama: mod.Wllama, wasmUrl: RT.wasm };
    });
}

function loadModel() {
    var key = el('tensor-model').value || Object.keys(MODELS)[0] || 'qwen15';
    var spec = MODELS[key] || MODELS.qwen15;
    if (!spec) { setStatus('no models available'); return; }
    setStatus('loading runtime&hellip;');
    loadWllama().then(function (rt) {
        setStatus('loading ' + esc(spec.label) + '&hellip;');
        startModelLoad(rt, spec, key);
    }).catch(function (e) {
        setStatus('runtime load failed: ' + esc(String((e && e.message) || e)));
        logRow('err', 'runtime import failed: ' + String((e && e.message) || e));
    });
}

function startModelLoad(rt, spec, key) {
    var inst = new rt.Wllama({ default: rt.wasmUrl });
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
    if (!S.wllama) { setStatus('load the model first'); return; }
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
    if (S.running) { return; }
    S.running = true;
    setStatus('worker up on ' + esc(S.peer));
    el('tensor-start').disabled = true;
    el('tensor-stop').disabled = false;
    var tick = function () {
        if (!S.running) { return; }
        pollOnce().then(function () {
            if (S.running) { S.timer = setTimeout(tick, POLL_MS); }
        });
    };
    tick();
}

function stopLoop() {
    S.running = false;
    if (S.timer) { clearTimeout(S.timer); S.timer = null; }
    el('tensor-start').disabled = false;
    el('tensor-stop').disabled = true;
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
el('tensor-self').addEventListener('click', selfTest);
window.__tensorReady = true;
// Catalog first (host vendor + GGUF library), so the list reflects what
// this box actually serves; HF fallback when the host has no catalog.
bootConfig();
