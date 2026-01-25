// Tools configuration page

const API_BASE = window.location.origin;

let currentConfig = null;
let tools = [];
let authToken = null;

// Get auth token or redirect to login
function getAuthToken() {
    const token = localStorage.getItem('stark_token');
    if (!token) {
        window.location.href = '/';
        return null;
    }
    return token;
}

// Make authenticated fetch request
async function authFetch(url, options = {}) {
    if (!authToken) {
        authToken = getAuthToken();
        if (!authToken) return null;
    }

    const headers = {
        ...options.headers,
        'Authorization': `Bearer ${authToken}`
    };

    const response = await fetch(url, { ...options, headers });

    if (response.status === 401) {
        localStorage.removeItem('stark_token');
        window.location.href = '/';
        return null;
    }

    return response;
}

// Group icons mapping
const groupIcons = {
    web: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"></path></svg>`,
    filesystem: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path></svg>`,
    exec: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>`,
    messaging: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path></svg>`,
    system: `<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path></svg>`
};

document.addEventListener('DOMContentLoaded', () => {
    authToken = getAuthToken();
    if (!authToken) return;

    loadTools();
    loadConfig();
    loadHistory();
    setupEventListeners();
});

function setupEventListeners() {
    // Profile buttons
    document.querySelectorAll('.profile-btn').forEach(btn => {
        btn.addEventListener('click', () => selectProfile(btn.dataset.profile));
    });

    // Save button
    document.getElementById('save-config').addEventListener('click', saveConfig);

    // Refresh history
    document.getElementById('refresh-history').addEventListener('click', loadHistory);

    // Logout
    document.getElementById('logout-btn').addEventListener('click', () => {
        window.location.href = '/index.html';
    });
}

async function loadTools() {
    try {
        const response = await authFetch(`${API_BASE}/api/tools`);
        if (!response || !response.ok) throw new Error('Failed to load tools');

        tools = await response.json();
        renderTools();
    } catch (error) {
        console.error('Error loading tools:', error);
        document.getElementById('tools-list').innerHTML =
            '<div class="text-red-400">Failed to load tools</div>';
    }
}

async function loadConfig() {
    try {
        const response = await authFetch(`${API_BASE}/api/tools/config`);
        if (!response || !response.ok) throw new Error('Failed to load config');

        currentConfig = await response.json();
        updateProfileButtons();
        renderTools();
    } catch (error) {
        console.error('Error loading config:', error);
        // Use default config
        currentConfig = {
            profile: 'standard',
            allow_list: [],
            deny_list: [],
            allowed_groups: ['web', 'filesystem'],
            denied_groups: []
        };
        updateProfileButtons();
    }
}

async function loadHistory() {
    try {
        const response = await authFetch(`${API_BASE}/api/tools/history?limit=50`);
        if (!response || !response.ok) throw new Error('Failed to load history');

        const history = await response.json();
        renderHistory(history);
    } catch (error) {
        console.error('Error loading history:', error);
        document.getElementById('history-list').innerHTML =
            '<div class="text-slate-500">No execution history available</div>';
    }
}

function renderTools() {
    if (!tools.length) {
        document.getElementById('tools-list').innerHTML =
            '<div class="text-slate-500">No tools registered</div>';
        return;
    }

    // Group tools by their group
    const grouped = {};
    tools.forEach(tool => {
        const group = tool.group || 'other';
        if (!grouped[group]) grouped[group] = [];
        grouped[group].push(tool);
    });

    let html = '';
    for (const [group, groupTools] of Object.entries(grouped)) {
        const icon = groupIcons[group] || groupIcons.system;
        html += `
            <div class="mb-4">
                <div class="flex items-center gap-2 mb-2 text-slate-300">
                    ${icon}
                    <span class="font-medium capitalize">${group}</span>
                </div>
                <div class="space-y-2 ml-7">
        `;

        groupTools.forEach(tool => {
            const isEnabled = isToolEnabled(tool.name, group);
            html += `
                <div class="flex items-center justify-between p-3 bg-slate-900 rounded-lg">
                    <div>
                        <div class="font-medium text-white">${tool.name}</div>
                        <div class="text-sm text-slate-500">${tool.description}</div>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                        <input type="checkbox" class="sr-only peer tool-toggle"
                               data-tool="${tool.name}" data-group="${group}"
                               ${isEnabled ? 'checked' : ''}>
                        <div class="w-11 h-6 bg-slate-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-stark-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-stark-500"></div>
                    </label>
                </div>
            `;
        });

        html += '</div></div>';
    }

    document.getElementById('tools-list').innerHTML = html;

    // Add toggle listeners
    document.querySelectorAll('.tool-toggle').forEach(toggle => {
        toggle.addEventListener('change', (e) => {
            toggleTool(e.target.dataset.tool, e.target.dataset.group, e.target.checked);
        });
    });
}

function isToolEnabled(toolName, group) {
    if (!currentConfig) return false;

    // Check explicit deny
    if (currentConfig.deny_list?.includes(toolName)) return false;

    // Check explicit allow
    if (currentConfig.allow_list?.includes(toolName)) return true;

    // Check group denial
    if (currentConfig.denied_groups?.includes(group)) return false;

    // Check profile or allowed groups
    const profileGroups = getProfileGroups(currentConfig.profile);
    if (currentConfig.profile === 'custom') {
        return currentConfig.allowed_groups?.includes(group) || false;
    }
    return profileGroups.includes(group);
}

function getProfileGroups(profile) {
    switch (profile) {
        case 'none': return [];
        case 'minimal': return ['web'];
        case 'standard': return ['web', 'filesystem'];
        case 'messaging': return ['web', 'filesystem', 'messaging'];
        case 'full': return ['web', 'filesystem', 'exec', 'messaging', 'system'];
        default: return [];
    }
}

function selectProfile(profile) {
    currentConfig.profile = profile;
    if (profile !== 'custom') {
        currentConfig.allowed_groups = getProfileGroups(profile);
        currentConfig.allow_list = [];
        currentConfig.deny_list = [];
    }
    updateProfileButtons();
    renderTools();
}

function updateProfileButtons() {
    document.querySelectorAll('.profile-btn').forEach(btn => {
        const isSelected = btn.dataset.profile === currentConfig.profile;
        if (isSelected) {
            btn.classList.add('border-stark-500', 'text-stark-400', 'bg-stark-500/10');
            btn.classList.remove('border-slate-600', 'text-slate-300');
        } else {
            btn.classList.remove('border-stark-500', 'text-stark-400', 'bg-stark-500/10');
            btn.classList.add('border-slate-600', 'text-slate-300');
        }
    });
}

function toggleTool(toolName, group, enabled) {
    // Switch to custom profile when manually toggling
    if (currentConfig.profile !== 'custom') {
        currentConfig.profile = 'custom';
        currentConfig.allowed_groups = getProfileGroups(currentConfig.profile);
        updateProfileButtons();
    }

    if (enabled) {
        // Remove from deny list, add to allow list
        currentConfig.deny_list = currentConfig.deny_list?.filter(t => t !== toolName) || [];
        if (!currentConfig.allow_list?.includes(toolName)) {
            currentConfig.allow_list = [...(currentConfig.allow_list || []), toolName];
        }
    } else {
        // Remove from allow list, add to deny list
        currentConfig.allow_list = currentConfig.allow_list?.filter(t => t !== toolName) || [];
        if (!currentConfig.deny_list?.includes(toolName)) {
            currentConfig.deny_list = [...(currentConfig.deny_list || []), toolName];
        }
    }
}

async function saveConfig() {
    try {
        const response = await authFetch(`${API_BASE}/api/tools/config`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(currentConfig)
        });

        if (!response || !response.ok) throw new Error('Failed to save config');

        showSuccess('Configuration saved successfully');
    } catch (error) {
        console.error('Error saving config:', error);
        showError('Failed to save configuration');
    }
}

function renderHistory(history) {
    if (!history.length) {
        document.getElementById('history-list').innerHTML =
            '<div class="text-slate-500">No execution history</div>';
        return;
    }

    const html = history.map(item => `
        <div class="flex items-center justify-between p-2 bg-slate-900 rounded text-sm">
            <div class="flex items-center gap-3">
                <span class="${item.success ? 'text-green-400' : 'text-red-400'}">
                    ${item.success ? '&#10003;' : '&#10007;'}
                </span>
                <span class="text-white font-medium">${item.tool_name}</span>
                <span class="text-slate-500 text-xs">${formatTime(item.executed_at)}</span>
            </div>
            <span class="text-slate-500 text-xs">${item.duration_ms || 0}ms</span>
        </div>
    `).join('');

    document.getElementById('history-list').innerHTML = html;
}

function formatTime(timestamp) {
    const date = new Date(timestamp);
    return date.toLocaleTimeString();
}

function showSuccess(message) {
    const el = document.getElementById('success-message');
    el.textContent = message;
    el.classList.remove('hidden');
    setTimeout(() => el.classList.add('hidden'), 3000);
}

function showError(message) {
    const el = document.getElementById('error-message');
    el.textContent = message;
    el.classList.remove('hidden');
    setTimeout(() => el.classList.add('hidden'), 5000);
}
