/* WebGPU adapter probe (swarm browser path, ticket t-1789411940089445600).
 *
 * Runs navigator.gpu.requestAdapter(), reads adapter info + allocation
 * limits + navigator.deviceMemory, renders them, and POSTs the raw report
 * to /api/edge/webgpu (Telegram initData handshake, same as board actions).
 * The server validates the payload and forwards a class=webgpu hub profile
 * to GSV — phones never touch poolAI/GSV directly.
 */
(function () {
    "use strict";

    function esc(s) {
        return String(s == null ? "" : s)
            .replace(/&/g, "&amp;").replace(/</g, "&lt;")
            .replace(/>/g, "&gt;").replace(/"/g, "&quot;");
    }

    function tg() {
        try { return (window.Telegram && window.Telegram.WebApp) || null; }
        catch (e) { return null; }
    }

    function userId() {
        try {
            var w = tg();
            if (w && w.initDataUnsafe && w.initDataUnsafe.user && w.initDataUnsafe.user.id) {
                return String(w.initDataUnsafe.user.id);
            }
        } catch (e) {}
        return "";
    }

    function platform() {
        try {
            var w = tg();
            if (w && w.platform) { return String(w.platform); }
        } catch (e) {}
        return "";
    }

    function setStatus(html) {
        var el = document.getElementById("probe-status");
        if (el) { el.innerHTML = html; }
    }

    function row(k, v) {
        return "<tr><td>" + esc(k) + "</td><td>" + esc(v) + "</td></tr>";
    }

    function timed(url, opts, ms) {
        if (typeof AbortController === "undefined" || typeof fetch === "undefined") {
            return fetch(url, opts);
        }
        var ctrl = new AbortController();
        var timer = setTimeout(function () { ctrl.abort(); }, ms || 20000);
        return fetch(url, Object.assign({}, opts, { signal: ctrl.signal }))
            .then(function (r) { clearTimeout(timer); return r; },
                  function (e) { clearTimeout(timer); throw e; });
    }

    function adapterInfo(adapter) {
        // New API (requestAdapterInfo) with fallback to the deprecated .info.
        try {
            if (typeof adapter.requestAdapterInfo === "function") {
                return adapter.requestAdapterInfo().then(function (i) { return i || {}; });
            }
        } catch (e) {}
        try {
            if (adapter.info) { return Promise.resolve(adapter.info); }
        } catch (e2) {}
        return Promise.resolve({});
    }

    function runProbe() {
        setStatus("probing&hellip;");
        if (!navigator.gpu) {
            return Promise.resolve({
                supported: false,
                reason: "navigator.gpu missing (WebView without WebGPU)"
            });
        }
        var p;
        try {
            // T2.1: core first (high-performance), then the compatibility
            // feature level — old Mali/Adreno WebViews (Redmi 9 class) return
            // null for core but serve a compat adapter. Unknown dict members
            // are ignored by old implementations, and every step is guarded.
            p = navigator.gpu.requestAdapter({ powerPreference: "high-performance" })
                .then(function (a) { return a || navigator.gpu.requestAdapter(); })
                .then(function (a) {
                    if (a) return { adapter: a, compat: false };
                    return navigator.gpu.requestAdapter({ featureLevel: "compatibility" })
                        .then(function (c) { return c ? { adapter: c, compat: true } : null; })
                        .catch(function () { return null; });
                });
        } catch (e) {
            return Promise.resolve({ supported: false, reason: "requestAdapter threw: " + String((e && e.name) || e) });
        }
        return p.then(function (found) {
            if (!found) {
                return { supported: false, reason: "requestAdapter() returned null" };
            }
            var adapter = found.adapter;
            var compat = !!found.compat;
            var core = false;
            try {
                core = !!(adapter.features && adapter.features.has &&
                    adapter.features.has("core-features-and-limits"));
            } catch (e2) { /* old WebView without features */ }
            return adapterInfo(adapter).then(function (info) {
                var lim = adapter.limits || {};
                var num = function (v) { return (typeof v === "number" && isFinite(v) && v >= 0) ? v : 0; };
                var devMem = (typeof navigator.deviceMemory === "number" &&
                              isFinite(navigator.deviceMemory) && navigator.deviceMemory > 0)
                    ? navigator.deviceMemory : null;
                return {
                    supported: true,
                    compat: compat,
                    core: core,
                    adapter: {
                        vendor: info.vendor || "",
                        architecture: info.architecture || "",
                        device: info.device || "",
                        description: info.description || ""
                    },
                    limits: {
                        maxStorageBufferBindingSize: num(lim.maxStorageBufferBindingSize),
                        maxBufferSize: num(lim.maxBufferSize),
                        maxTextureDimension2D: num(lim.maxTextureDimension2D)
                    },
                    deviceMemoryGb: devMem,
                    platform: platform()
                };
            });
        }).catch(function (e) {
            return { supported: false, reason: "probe failed: " + String((e && e.name) || e) };
        });
    }

    function render(report) {
        var body = document.getElementById("probe-body");
        if (!body) { return; }
        if (!report.supported) {
            body.innerHTML = row("WebGPU", "UNSUPPORTED") + row("reason", report.reason || "?");
            return;
        }
        var a = report.adapter || {};
        var l = report.limits || {};
        var mb = function (b) { return b ? (b / 1048576) + " MB" : "unknown"; };
        body.innerHTML =
            row("WebGPU", "SUPPORTED") +
            row("mode", report.compat ? "compatibility" : (report.core ? "core" : "default")) +
            row("vendor", a.vendor || "?") +
            row("architecture", a.architecture || "?") +
            row("device", a.device || "?") +
            row("max storage buffer", mb(l.maxStorageBufferBindingSize)) +
            row("max buffer", mb(l.maxBufferSize)) +
            row("max texture 2D", l.maxTextureDimension2D || "unknown") +
            row("device memory", report.deviceMemoryGb ? (report.deviceMemoryGb + " GB") : "unknown");
    }

    function submit(report) {
        var user = userId();
        if (!user) {
            setStatus("Open this page from Telegram so the probe can identify your peer.");
            return;
        }
        var w = tg();
        var initData = (w && w.initData) || "";
        var authDate = Math.floor(Date.now() / 1000);
        var call;
        try {
            call = timed("/api/edge/webgpu?initData=" + encodeURIComponent(initData) +
                "&authDate=" + authDate, {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ user: user, probe: report })
                }, 20000);
        } catch (e) {
            setStatus("fetch failed: " + esc(String((e && e.name) || e)));
            return;
        }
        setStatus("registering hub profile&hellip;");
        call.then(function (r) { return r.json(); })
            .then(function (data) {
                if (data && data.ok) {
                    if (data.unsupported) {
                        setStatus("recorded: adapter unsupported on this device.");
                    } else {
                        setStatus("hub profile upserted: <code>" +
                            esc((data.profile && data.profile.id) || "?") +
                            "</code> class=webgpu ram_mb=" +
                            esc((data.profile && data.profile.ram_mb) || 0));
                    }
                } else {
                    setStatus("error: " + esc((data && data.error) || "unreachable"));
                }
            })
            .catch(function () { setStatus("Telenetis unreachable"); });
    }

    document.getElementById("probe-run").addEventListener("click", function () {
        runProbe().then(function (report) {
            render(report);
            submit(report);
        });
    });
})();
