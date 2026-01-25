/**
 * Debug page - System diagnostics and debugging tools
 */

let gateway = null;
let startTime = Date.now();

document.addEventListener('DOMContentLoaded', function() {
    const token = localStorage.getItem('stark_token');

    if (!token) {
        window.location.href = '/';
        return;
    }

    initializeGateway();
    setupEventListeners();
    updateSystemInfo();
    startUptimeCounter();
});

function initializeGateway() {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsPort = 8766; // Gateway port
    const wsUrl = `${wsProtocol}//${window.location.hostname}:${wsPort}`;

    document.getElementById('gateway-url').textContent = wsUrl;

    gateway = new GatewayClient(wsUrl);

    gateway.onConnectionChange = (connected) => {
        updateGatewayStatus(connected);
        if (connected) {
            fetchStatus();
            fetchChannelStatus();
        }
    };

    gateway.connect().catch(err => {
        console.error('Failed to connect to Gateway:', err);
        updateGatewayStatus(false);
    });
}

function updateGatewayStatus(connected) {
    const statusIcon = document.getElementById('gateway-status-icon');
    const statusText = document.getElementById('gateway-status');

    if (connected) {
        statusIcon.className = 'w-12 h-12 bg-green-500/20 rounded-lg flex items-center justify-center';
        statusIcon.innerHTML = '<div class="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>';
        statusText.textContent = 'Connected';
        statusText.className = 'text-xl font-semibold text-green-400';
    } else {
        statusIcon.className = 'w-12 h-12 bg-red-500/20 rounded-lg flex items-center justify-center';
        statusIcon.innerHTML = '<div class="w-3 h-3 bg-red-500 rounded-full"></div>';
        statusText.textContent = 'Disconnected';
        statusText.className = 'text-xl font-semibold text-red-400';
    }
}

async function fetchStatus() {
    try {
        const result = await gateway.call('status');
        document.getElementById('client-count').textContent = result.connected_clients || 0;
    } catch (err) {
        console.error('Failed to fetch status:', err);
    }
}

async function fetchChannelStatus() {
    const container = document.getElementById('channels-list');

    try {
        const result = await gateway.call('channels.status');

        if (!result.channels || result.channels.length === 0) {
            container.innerHTML = '<div class="text-slate-500 text-center py-4">No channels configured</div>';
            return;
        }

        container.innerHTML = result.channels.map(channel => `
            <div class="flex items-center justify-between p-4 bg-slate-900 rounded-lg">
                <div class="flex items-center gap-4">
                    <div class="w-10 h-10 rounded-lg flex items-center justify-center ${channel.is_running ? 'bg-green-500/20' : 'bg-slate-700'}">
                        ${getChannelIcon(channel.channel_type)}
                    </div>
                    <div>
                        <p class="font-medium text-white">${channel.name}</p>
                        <p class="text-sm text-slate-500">${channel.channel_type} #${channel.id}</p>
                    </div>
                </div>
                <div class="flex items-center gap-3">
                    <span class="px-3 py-1 rounded-full text-xs font-medium ${channel.is_running ? 'bg-green-500/20 text-green-400' : 'bg-slate-700 text-slate-400'}">
                        ${channel.is_running ? 'Running' : 'Stopped'}
                    </span>
                    <button onclick="toggleChannel(${channel.id}, ${channel.is_running})" class="px-3 py-1 rounded-lg text-xs font-medium ${channel.is_running ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30' : 'bg-green-500/20 text-green-400 hover:bg-green-500/30'} transition-colors">
                        ${channel.is_running ? 'Stop' : 'Start'}
                    </button>
                </div>
            </div>
        `).join('');

    } catch (err) {
        console.error('Failed to fetch channel status:', err);
        container.innerHTML = '<div class="text-red-400 text-center py-4">Failed to load channels</div>';
    }
}

function getChannelIcon(type) {
    const icons = {
        'telegram': '<svg class="w-5 h-5 text-blue-400" fill="currentColor" viewBox="0 0 24 24"><path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/></svg>',
        'slack': '<svg class="w-5 h-5 text-purple-400" fill="currentColor" viewBox="0 0 24 24"><path d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zM6.313 15.165a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zM8.834 5.042a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zM8.834 6.313a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zM18.956 8.834a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zM17.688 8.834a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zM15.165 18.956a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zM15.165 17.688a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"/></svg>',
    };
    return icons[type] || '<svg class="w-5 h-5 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path></svg>';
}

async function toggleChannel(id, isRunning) {
    try {
        if (isRunning) {
            await gateway.call('channels.stop', { id });
        } else {
            await gateway.call('channels.start', { id });
        }
        // Refresh status after a short delay
        setTimeout(fetchChannelStatus, 500);
    } catch (err) {
        console.error('Failed to toggle channel:', err);
        alert('Failed to toggle channel: ' + err.message);
    }
}

async function sendRpcRequest() {
    const method = document.getElementById('rpc-method').value;
    const paramsText = document.getElementById('rpc-params').value;
    const resultDiv = document.getElementById('rpc-result');
    const statusSpan = document.getElementById('rpc-status');
    const preElement = resultDiv.querySelector('pre');

    let params = {};
    if (paramsText.trim()) {
        try {
            params = JSON.parse(paramsText);
        } catch (e) {
            preElement.textContent = 'Invalid JSON in params';
            statusSpan.textContent = 'Error';
            statusSpan.className = 'text-xs px-2 py-1 rounded-full bg-red-500/20 text-red-400';
            resultDiv.classList.remove('hidden');
            return;
        }
    }

    try {
        const startTime = performance.now();
        const result = await gateway.call(method, params);
        const duration = (performance.now() - startTime).toFixed(2);

        preElement.textContent = JSON.stringify(result, null, 2);
        statusSpan.textContent = `Success (${duration}ms)`;
        statusSpan.className = 'text-xs px-2 py-1 rounded-full bg-green-500/20 text-green-400';
    } catch (err) {
        preElement.textContent = err.message;
        statusSpan.textContent = 'Error';
        statusSpan.className = 'text-xs px-2 py-1 rounded-full bg-red-500/20 text-red-400';
    }

    resultDiv.classList.remove('hidden');
}

function updateSystemInfo() {
    // Browser info
    const browserInfo = navigator.userAgent.split(' ').slice(-2).join(' ');
    document.getElementById('browser-info').textContent = browserInfo;

    // Auth status
    const token = localStorage.getItem('stark_token');
    if (token) {
        document.getElementById('auth-status').textContent = token.slice(0, 8) + '...';
    } else {
        document.getElementById('auth-status').textContent = 'Not authenticated';
    }

    // Update time every second
    updateLocalTime();
    setInterval(updateLocalTime, 1000);
}

function updateLocalTime() {
    document.getElementById('local-time').textContent = new Date().toLocaleTimeString();
}

function startUptimeCounter() {
    function updateUptime() {
        const elapsed = Date.now() - startTime;
        const seconds = Math.floor(elapsed / 1000) % 60;
        const minutes = Math.floor(elapsed / 60000) % 60;
        const hours = Math.floor(elapsed / 3600000);

        let uptimeStr = '';
        if (hours > 0) uptimeStr += `${hours}h `;
        if (minutes > 0 || hours > 0) uptimeStr += `${minutes}m `;
        uptimeStr += `${seconds}s`;

        document.getElementById('uptime').textContent = uptimeStr;
    }

    updateUptime();
    setInterval(updateUptime, 1000);
}

function clearLocalStorage() {
    if (confirm('This will clear all local storage and log you out. Continue?')) {
        localStorage.clear();
        window.location.href = '/';
    }
}

function copyToken() {
    const token = localStorage.getItem('stark_token');
    if (token) {
        navigator.clipboard.writeText(token).then(() => {
            alert('Token copied to clipboard');
        }).catch(() => {
            prompt('Copy this token:', token);
        });
    } else {
        alert('No token found');
    }
}

function triggerTestEvent() {
    // This is a client-side simulation - in a real app you'd call an API
    if (gateway && gateway.isConnected()) {
        alert('Test event triggered - check Logs page to see events from channels');
    } else {
        alert('Not connected to Gateway');
    }
}

function setupEventListeners() {
    // Logout
    document.getElementById('logout-btn').addEventListener('click', () => {
        localStorage.removeItem('stark_token');
        window.location.href = '/';
    });

    // RPC tester
    document.getElementById('send-rpc').addEventListener('click', sendRpcRequest);
    document.getElementById('rpc-params').addEventListener('keypress', (e) => {
        if (e.key === 'Enter') sendRpcRequest();
    });

    // Refresh channels
    document.getElementById('refresh-channels').addEventListener('click', fetchChannelStatus);

    // Quick actions
    document.getElementById('clear-storage').addEventListener('click', clearLocalStorage);
    document.getElementById('copy-token').addEventListener('click', copyToken);
    document.getElementById('test-event').addEventListener('click', triggerTestEvent);
}
