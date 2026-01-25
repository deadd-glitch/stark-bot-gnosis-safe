/**
 * Sessions management page JavaScript
 */

const API_BASE = '/api';

/**
 * Initialize the page
 */
async function init() {
    const token = localStorage.getItem('stark_token');
    if (!token) {
        window.location.href = '/';
        return;
    }

    // Validate session
    try {
        const response = await fetch(`${API_BASE}/auth/validate`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();
        if (!data.valid) {
            localStorage.removeItem('stark_token');
            window.location.href = '/';
            return;
        }
    } catch (error) {
        console.error('Session validation error:', error);
        localStorage.removeItem('stark_token');
        window.location.href = '/';
        return;
    }

    setupEventListeners();
    loadSessions();
}

/**
 * Setup event listeners
 */
function setupEventListeners() {
    document.getElementById('logout-btn').addEventListener('click', async () => {
        const token = localStorage.getItem('stark_token');
        try {
            await fetch(`${API_BASE}/auth/logout`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ token })
            });
        } catch (error) {
            console.error('Logout error:', error);
        }
        localStorage.removeItem('stark_token');
        window.location.href = '/';
    });

    document.getElementById('refresh-btn').addEventListener('click', () => {
        loadSessions();
    });

    document.getElementById('close-modal').addEventListener('click', () => {
        document.getElementById('session-modal').classList.add('hidden');
    });

    document.getElementById('session-modal').addEventListener('click', (e) => {
        if (e.target.id === 'session-modal') {
            document.getElementById('session-modal').classList.add('hidden');
        }
    });
}

/**
 * Load sessions from API
 */
async function loadSessions() {
    const token = localStorage.getItem('stark_token');
    const loading = document.getElementById('loading');
    const sessionsList = document.getElementById('sessions-list');
    const noSessions = document.getElementById('no-sessions');

    loading.classList.remove('hidden');
    sessionsList.classList.add('hidden');
    noSessions.classList.add('hidden');

    try {
        // Get channels first to get their sessions
        const channelsResponse = await fetch(`${API_BASE}/channels`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const channelsData = await channelsResponse.json();
        const channels = channelsData.channels || [];

        // For now, we'll display a placeholder since we need to query sessions
        // In a real implementation, we'd have an endpoint to list all sessions
        loading.classList.add('hidden');

        // Update stats
        document.getElementById('stat-channels').textContent = channels.length;

        // Show placeholder for now
        if (channels.length === 0) {
            noSessions.classList.remove('hidden');
            document.getElementById('stat-active').textContent = '0';
            document.getElementById('stat-messages').textContent = '0';
        } else {
            sessionsList.classList.remove('hidden');
            sessionsList.innerHTML = `
                <div class="text-slate-400 text-center py-8">
                    <p>Sessions are created automatically when users interact with the bot.</p>
                    <p class="text-sm mt-2">To view a specific session, use the API endpoint:</p>
                    <code class="block mt-2 bg-slate-900 px-4 py-2 rounded text-stark-400">POST /api/sessions</code>
                </div>
            `;
            document.getElementById('stat-active').textContent = '-';
            document.getElementById('stat-messages').textContent = '-';
        }
    } catch (error) {
        console.error('Failed to load sessions:', error);
        loading.classList.add('hidden');
        showError('Failed to load sessions');
    }
}

/**
 * View session transcript
 */
async function viewTranscript(sessionId) {
    const token = localStorage.getItem('stark_token');
    const modal = document.getElementById('session-modal');
    const content = document.getElementById('modal-content');

    content.innerHTML = '<div class="text-slate-400">Loading transcript...</div>';
    modal.classList.remove('hidden');

    try {
        const response = await fetch(`${API_BASE}/sessions/${sessionId}/transcript`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        if (data.messages && data.messages.length > 0) {
            content.innerHTML = data.messages.map(msg => `
                <div class="p-3 rounded-lg ${msg.role === 'user' ? 'bg-slate-700' : 'bg-stark-500/20'}">
                    <div class="flex items-center gap-2 mb-1">
                        <span class="font-medium ${msg.role === 'user' ? 'text-white' : 'text-stark-400'}">
                            ${msg.role === 'user' ? (msg.user_name || 'User') : 'Assistant'}
                        </span>
                        <span class="text-xs text-slate-500">${formatDate(msg.created_at)}</span>
                    </div>
                    <p class="text-slate-300">${escapeHtml(msg.content)}</p>
                </div>
            `).join('');
        } else {
            content.innerHTML = '<div class="text-slate-500 text-center py-4">No messages in this session.</div>';
        }
    } catch (error) {
        console.error('Failed to load transcript:', error);
        content.innerHTML = '<div class="text-red-400">Failed to load transcript.</div>';
    }
}

/**
 * Reset a session
 */
async function resetSession(sessionId) {
    if (!confirm('Are you sure you want to reset this session? This will clear the conversation history.')) {
        return;
    }

    const token = localStorage.getItem('stark_token');

    try {
        const response = await fetch(`${API_BASE}/sessions/${sessionId}/reset`, {
            method: 'POST',
            headers: { 'Authorization': `Bearer ${token}` }
        });

        if (response.ok) {
            showSuccess('Session reset successfully');
            loadSessions();
        } else {
            showError('Failed to reset session');
        }
    } catch (error) {
        console.error('Failed to reset session:', error);
        showError('Failed to reset session');
    }
}

/**
 * Show success message
 */
function showSuccess(message) {
    const el = document.getElementById('success-message');
    el.textContent = message;
    el.classList.remove('hidden');
    document.getElementById('error-message').classList.add('hidden');
    setTimeout(() => el.classList.add('hidden'), 5000);
}

/**
 * Show error message
 */
function showError(message) {
    const el = document.getElementById('error-message');
    el.textContent = message;
    el.classList.remove('hidden');
    document.getElementById('success-message').classList.add('hidden');
    setTimeout(() => el.classList.add('hidden'), 5000);
}

/**
 * Utility functions
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function formatDate(dateStr) {
    const date = new Date(dateStr);
    return date.toLocaleString();
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', init);
