/**
 * Logs page - Real-time event viewer
 */

let gateway = null;
let autoScroll = true;
let currentFilter = 'all';
let logs = [];
let counts = {
    total: 0,
    channel: 0,
    agent: 0,
    tool: 0,
    skill: 0,
    error: 0
};

const eventColors = {
    'channel.started': 'text-green-400',
    'channel.stopped': 'text-yellow-400',
    'channel.error': 'text-red-400',
    'channel.message': 'text-blue-400',
    'agent.response': 'text-emerald-400',
    'tool.execution': 'text-purple-400',
    'tool.result': 'text-violet-400',
    'skill.invoked': 'text-pink-400',
};

const eventIcons = {
    'channel.started': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>',
    'channel.stopped': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z"></path></svg>',
    'channel.error': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>',
    'channel.message': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path></svg>',
    'agent.response': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path></svg>',
    'tool.execution': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>',
    'tool.result': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>',
    'skill.invoked': '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path></svg>',
};

document.addEventListener('DOMContentLoaded', function() {
    const token = localStorage.getItem('stark_token');

    if (!token) {
        window.location.href = '/';
        return;
    }

    initializeGateway();
    setupEventListeners();
});

function initializeGateway() {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsPort = 8766; // Gateway port
    const wsUrl = `${wsProtocol}//${window.location.hostname}:${wsPort}`;

    gateway = new GatewayClient(wsUrl);

    gateway.onConnectionChange = (connected) => {
        updateConnectionStatus(connected);
    };

    // Subscribe to all events
    gateway.on('*', (eventName, data) => {
        addLogEntry(eventName, data);
    });

    gateway.connect().catch(err => {
        console.error('Failed to connect to Gateway:', err);
        updateConnectionStatus(false);
    });
}

function updateConnectionStatus(connected) {
    const statusDot = document.getElementById('status-dot');
    const statusText = document.getElementById('status-text');

    if (connected) {
        statusDot.className = 'w-2 h-2 rounded-full bg-green-500 animate-pulse';
        statusText.textContent = 'Connected';
        statusText.className = 'text-sm text-green-400';
    } else {
        statusDot.className = 'w-2 h-2 rounded-full bg-red-500';
        statusText.textContent = 'Disconnected';
        statusText.className = 'text-sm text-red-400';
    }
}

function addLogEntry(eventName, data) {
    const timestamp = new Date().toISOString();
    const entry = { timestamp, event: eventName, data };
    logs.push(entry);

    // Update counts
    counts.total++;
    if (eventName.startsWith('channel.')) counts.channel++;
    if (eventName.startsWith('agent.')) counts.agent++;
    if (eventName.startsWith('tool.')) counts.tool++;
    if (eventName.startsWith('skill.')) counts.skill++;
    if (eventName.includes('error')) counts.error++;

    updateCountDisplay();

    // Check if entry matches current filter
    if (shouldShowEntry(eventName)) {
        renderLogEntry(entry);
    }

    // Update empty state
    const emptyState = document.getElementById('empty-state');
    if (logs.length > 0) {
        emptyState.classList.add('hidden');
    }

    // Update last event time
    document.getElementById('last-event-time').textContent = `Last event: ${formatTime(timestamp)}`;

    // Auto-scroll if enabled
    if (autoScroll) {
        const container = document.getElementById('log-container');
        container.scrollTop = container.scrollHeight;
    }
}

function shouldShowEntry(eventName) {
    if (currentFilter === 'all') return true;
    if (currentFilter === 'error') return eventName.includes('error');
    return eventName.startsWith(currentFilter + '.');
}

function renderLogEntry(entry) {
    const container = document.getElementById('log-entries');
    const colorClass = eventColors[entry.event] || 'text-slate-400';
    const icon = eventIcons[entry.event] || '<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" stroke-width="2"/></svg>';

    const div = document.createElement('div');
    div.className = 'log-entry flex items-start gap-3 py-2 px-3 hover:bg-slate-800/50 rounded transition-colors';
    div.innerHTML = `
        <span class="text-slate-600 text-xs whitespace-nowrap">${formatTime(entry.timestamp)}</span>
        <span class="${colorClass} flex-shrink-0">${icon}</span>
        <span class="${colorClass} font-medium whitespace-nowrap">${entry.event}</span>
        <span class="text-slate-500 truncate flex-1">${formatData(entry.data)}</span>
    `;

    // Add click to expand
    div.addEventListener('click', () => {
        showLogDetail(entry);
    });

    container.appendChild(div);
}

function formatData(data) {
    if (!data) return '';

    // Format based on event type
    if (data.text) {
        return truncate(data.text, 100);
    }
    if (data.from) {
        return `from: ${data.from}`;
    }
    if (data.tool_name) {
        return `tool: ${data.tool_name}`;
    }
    if (data.skill_name) {
        return `skill: ${data.skill_name}`;
    }
    if (data.name) {
        return data.name;
    }
    if (data.error) {
        return `Error: ${truncate(data.error, 80)}`;
    }

    return JSON.stringify(data).slice(0, 100);
}

function truncate(str, maxLen) {
    if (str.length <= maxLen) return str;
    return str.slice(0, maxLen) + '...';
}

function formatTime(isoString) {
    const date = new Date(isoString);
    return date.toLocaleTimeString('en-US', {
        hour12: false,
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        fractionalSecondDigits: 3
    });
}

function showLogDetail(entry) {
    // Create modal
    const modal = document.createElement('div');
    modal.className = 'fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4';
    modal.onclick = (e) => {
        if (e.target === modal) modal.remove();
    };

    const colorClass = eventColors[entry.event] || 'text-slate-400';

    modal.innerHTML = `
        <div class="bg-slate-800 border border-slate-700 rounded-xl max-w-2xl w-full max-h-[80vh] overflow-hidden">
            <div class="p-4 border-b border-slate-700 flex items-center justify-between">
                <div>
                    <span class="${colorClass} font-semibold">${entry.event}</span>
                    <span class="text-slate-500 text-sm ml-2">${formatTime(entry.timestamp)}</span>
                </div>
                <button class="text-slate-400 hover:text-white" onclick="this.closest('.fixed').remove()">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                </button>
            </div>
            <div class="p-4 overflow-auto max-h-[60vh]">
                <pre class="json-viewer text-slate-300 whitespace-pre-wrap">${JSON.stringify(entry.data, null, 2)}</pre>
            </div>
        </div>
    `;

    document.body.appendChild(modal);
}

function updateCountDisplay() {
    document.getElementById('total-count').textContent = counts.total;
    document.getElementById('channel-count').textContent = counts.channel;
    document.getElementById('agent-count').textContent = counts.agent;
    document.getElementById('tool-count').textContent = counts.tool;
    document.getElementById('error-count').textContent = counts.error;
}

function clearLogs() {
    logs = [];
    counts = { total: 0, channel: 0, agent: 0, tool: 0, skill: 0, error: 0 };

    document.getElementById('log-entries').innerHTML = '';
    document.getElementById('empty-state').classList.remove('hidden');
    document.getElementById('last-event-time').textContent = 'No events yet';
    updateCountDisplay();
}

function setFilter(filter) {
    currentFilter = filter;

    // Update button styles
    document.querySelectorAll('.filter-btn').forEach(btn => {
        if (btn.dataset.filter === filter) {
            btn.className = 'filter-btn px-3 py-1 rounded-full text-sm bg-stark-500 text-white';
        } else {
            btn.className = 'filter-btn px-3 py-1 rounded-full text-sm bg-slate-700 text-slate-300 hover:bg-slate-600';
        }
    });

    // Re-render logs with filter
    const container = document.getElementById('log-entries');
    container.innerHTML = '';

    logs.filter(entry => shouldShowEntry(entry.event)).forEach(entry => {
        renderLogEntry(entry);
    });

    // Update empty state
    const emptyState = document.getElementById('empty-state');
    if (logs.filter(entry => shouldShowEntry(entry.event)).length === 0) {
        emptyState.classList.remove('hidden');
    } else {
        emptyState.classList.add('hidden');
    }
}

function toggleAutoScroll() {
    autoScroll = !autoScroll;
    const btn = document.getElementById('toggle-autoscroll');

    if (autoScroll) {
        btn.textContent = 'Auto-scroll: ON';
        btn.className = 'px-4 py-2 bg-stark-500/20 text-stark-400 hover:bg-stark-500/30 rounded-lg transition-colors text-sm';
    } else {
        btn.textContent = 'Auto-scroll: OFF';
        btn.className = 'px-4 py-2 bg-slate-700 text-slate-400 hover:bg-slate-600 rounded-lg transition-colors text-sm';
    }
}

function setupEventListeners() {
    // Logout
    document.getElementById('logout-btn').addEventListener('click', () => {
        localStorage.removeItem('stark_token');
        window.location.href = '/';
    });

    // Clear logs
    document.getElementById('clear-logs').addEventListener('click', clearLogs);

    // Toggle auto-scroll
    document.getElementById('toggle-autoscroll').addEventListener('click', toggleAutoScroll);

    // Filter buttons
    document.querySelectorAll('.filter-btn').forEach(btn => {
        btn.addEventListener('click', () => setFilter(btn.dataset.filter));
    });
}
