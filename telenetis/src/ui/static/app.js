const WS_URL = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws`;

function initTelegramWebApp() {
    if (window.Telegram && window.Telegram.WebApp) {
        const tg = window.Telegram.WebApp;
        tg.expand();
        tg.ready();
        tg.setHeaderColor && tg.setHeaderColor('#1c2733');
    }
}

function connectWS() {
    const ws = new WebSocket(WS_URL);
    const log = document.getElementById('flow-log');

    ws.onmessage = function(event) {
        const data = JSON.parse(event.data);
        if (log) {
            const entry = document.createElement('div');
            entry.className = 'flow-entry';
            entry.innerHTML = `<span class="flow-ts">${data.ts}</span><strong>${data.jail_id}</strong> ${data.action}: ${data.detail}`;
            log.prepend(entry);
        }
    };

    ws.onclose = function() {
        setTimeout(connectWS, 3000);
    };
}

async function loadStatus() {
    try {
        const resp = await fetch('/api/status');
        const data = await resp.json();
        const el = document.getElementById('status');
        if (el) {
            el.textContent = `Online: ${data.online} | Tickets: ${data.tickets_count} | Workers: ${data.workers_online}`;
        }
    } catch (e) {
        console.error('Failed to load status:', e);
    }
}

document.addEventListener('DOMContentLoaded', function() {
    initTelegramWebApp();
    connectWS();
    loadStatus();
    setInterval(loadStatus, 30000);
});
