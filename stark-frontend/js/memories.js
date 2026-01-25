/**
 * Memories management page JavaScript
 */

const API_BASE = '/api';

let currentTab = 'daily';

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
    loadDailyLogs();
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

    // Tab switching
    document.getElementById('tab-daily').addEventListener('click', () => switchTab('daily'));
    document.getElementById('tab-longterm').addEventListener('click', () => switchTab('longterm'));
    document.getElementById('tab-search').addEventListener('click', () => switchTab('search'));

    // Refresh buttons
    document.getElementById('refresh-daily').addEventListener('click', loadDailyLogs);
    document.getElementById('refresh-longterm').addEventListener('click', loadLongTermMemories);

    // Importance filter
    document.getElementById('importance-filter').addEventListener('change', loadLongTermMemories);

    // Search
    document.getElementById('search-btn').addEventListener('click', searchMemories);
    document.getElementById('search-query').addEventListener('keypress', (e) => {
        if (e.key === 'Enter') searchMemories();
    });

    // Cleanup
    document.getElementById('cleanup-btn').addEventListener('click', cleanupExpired);
}

/**
 * Switch between tabs
 */
function switchTab(tab) {
    currentTab = tab;

    // Update tab buttons
    const tabs = ['daily', 'longterm', 'search'];
    tabs.forEach(t => {
        const btn = document.getElementById(`tab-${t}`);
        const section = document.getElementById(`section-${t}`);

        if (t === tab) {
            btn.classList.remove('bg-slate-700', 'text-slate-300', 'hover:bg-slate-600');
            btn.classList.add('bg-stark-500/20', 'text-stark-400');
            section.classList.remove('hidden');
        } else {
            btn.classList.add('bg-slate-700', 'text-slate-300', 'hover:bg-slate-600');
            btn.classList.remove('bg-stark-500/20', 'text-stark-400');
            section.classList.add('hidden');
        }
    });

    // Load data for the tab
    if (tab === 'daily') loadDailyLogs();
    else if (tab === 'longterm') loadLongTermMemories();
}

/**
 * Load daily logs
 */
async function loadDailyLogs() {
    const token = localStorage.getItem('stark_token');
    const loading = document.getElementById('daily-loading');
    const list = document.getElementById('daily-list');
    const empty = document.getElementById('daily-empty');

    loading.classList.remove('hidden');
    list.classList.add('hidden');
    empty.classList.add('hidden');

    try {
        const response = await fetch(`${API_BASE}/memories/daily`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        loading.classList.add('hidden');

        if (Array.isArray(data) && data.length > 0) {
            list.classList.remove('hidden');
            list.innerHTML = data.map(memory => renderMemoryCard(memory)).join('');
        } else {
            empty.classList.remove('hidden');
        }
    } catch (error) {
        console.error('Failed to load daily logs:', error);
        loading.classList.add('hidden');
        showError('Failed to load daily logs');
    }
}

/**
 * Load long-term memories
 */
async function loadLongTermMemories() {
    const token = localStorage.getItem('stark_token');
    const loading = document.getElementById('longterm-loading');
    const list = document.getElementById('longterm-list');
    const empty = document.getElementById('longterm-empty');
    const minImportance = document.getElementById('importance-filter').value;

    loading.classList.remove('hidden');
    list.classList.add('hidden');
    empty.classList.add('hidden');

    try {
        let url = `${API_BASE}/memories/long-term?limit=50`;
        if (minImportance) {
            url += `&min_importance=${minImportance}`;
        }

        const response = await fetch(url, {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        loading.classList.add('hidden');

        if (Array.isArray(data) && data.length > 0) {
            list.classList.remove('hidden');
            list.innerHTML = data.map(memory => renderMemoryCard(memory)).join('');
        } else {
            empty.classList.remove('hidden');
        }
    } catch (error) {
        console.error('Failed to load long-term memories:', error);
        loading.classList.add('hidden');
        showError('Failed to load long-term memories');
    }
}

/**
 * Search memories
 */
async function searchMemories() {
    const token = localStorage.getItem('stark_token');
    const query = document.getElementById('search-query').value.trim();
    const results = document.getElementById('search-results');
    const empty = document.getElementById('search-empty');
    const placeholder = document.getElementById('search-placeholder');

    if (!query) {
        showError('Please enter a search query');
        return;
    }

    results.classList.add('hidden');
    empty.classList.add('hidden');
    placeholder.classList.add('hidden');
    results.innerHTML = '<div class="text-slate-400">Searching...</div>';
    results.classList.remove('hidden');

    try {
        const response = await fetch(`${API_BASE}/memories/search`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${token}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ query, limit: 50 })
        });
        const data = await response.json();

        if (Array.isArray(data) && data.length > 0) {
            results.innerHTML = data.map(result => renderMemoryCard(result.memory, result.rank)).join('');
        } else {
            results.classList.add('hidden');
            empty.classList.remove('hidden');
        }
    } catch (error) {
        console.error('Failed to search memories:', error);
        results.classList.add('hidden');
        showError('Failed to search memories');
    }
}

/**
 * Render a memory card
 */
function renderMemoryCard(memory, rank = null) {
    const importanceColors = {
        high: 'bg-red-500/20 text-red-400 border-red-500/50',
        medium: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/50',
        low: 'bg-green-500/20 text-green-400 border-green-500/50'
    };

    const importanceLevel = memory.importance >= 8 ? 'high' : (memory.importance >= 5 ? 'medium' : 'low');
    const colorClass = importanceColors[importanceLevel];

    return `
        <div class="p-4 bg-slate-900 rounded-lg">
            <div class="flex items-start justify-between gap-4">
                <div class="flex-1">
                    <p class="text-white mb-2">${escapeHtml(memory.content)}</p>
                    <div class="flex flex-wrap items-center gap-2 text-xs text-slate-500">
                        <span class="px-2 py-1 ${colorClass} rounded border">
                            Importance: ${memory.importance}
                        </span>
                        <span>${memory.memory_type === 'daily_log' ? 'Daily Log' : 'Long-term'}</span>
                        ${memory.category ? `<span>Category: ${escapeHtml(memory.category)}</span>` : ''}
                        ${memory.log_date ? `<span>Date: ${memory.log_date}</span>` : ''}
                        <span>${formatDate(memory.created_at)}</span>
                        ${rank !== null ? `<span class="text-stark-400">Relevance: ${Math.abs(rank).toFixed(2)}</span>` : ''}
                    </div>
                </div>
                <button onclick="deleteMemory(${memory.id})" class="px-3 py-1.5 bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded-lg text-sm font-medium transition-colors">
                    Delete
                </button>
            </div>
        </div>
    `;
}

/**
 * Delete a memory
 */
async function deleteMemory(id) {
    if (!confirm('Are you sure you want to delete this memory?')) {
        return;
    }

    const token = localStorage.getItem('stark_token');

    try {
        const response = await fetch(`${API_BASE}/memories/${id}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${token}` }
        });

        if (response.ok) {
            showSuccess('Memory deleted');
            // Reload current tab
            if (currentTab === 'daily') loadDailyLogs();
            else if (currentTab === 'longterm') loadLongTermMemories();
        } else {
            showError('Failed to delete memory');
        }
    } catch (error) {
        console.error('Failed to delete memory:', error);
        showError('Failed to delete memory');
    }
}

/**
 * Cleanup expired memories
 */
async function cleanupExpired() {
    if (!confirm('This will permanently delete all expired memories. Continue?')) {
        return;
    }

    const token = localStorage.getItem('stark_token');

    try {
        const response = await fetch(`${API_BASE}/memories/cleanup`, {
            method: 'POST',
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        if (data.success) {
            showSuccess(`Cleaned up ${data.deleted_count} expired memories`);
            // Reload current view
            if (currentTab === 'daily') loadDailyLogs();
            else if (currentTab === 'longterm') loadLongTermMemories();
        } else {
            showError('Failed to cleanup memories');
        }
    } catch (error) {
        console.error('Failed to cleanup memories:', error);
        showError('Failed to cleanup memories');
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
