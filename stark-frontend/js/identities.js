/**
 * Identities management page JavaScript
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

    document.getElementById('lookup-btn').addEventListener('click', lookupIdentity);
    document.getElementById('link-btn').addEventListener('click', linkIdentity);

    // Enter key handlers
    document.getElementById('lookup-user-id').addEventListener('keypress', (e) => {
        if (e.key === 'Enter') lookupIdentity();
    });
    document.getElementById('link-user-id').addEventListener('keypress', (e) => {
        if (e.key === 'Enter') linkIdentity();
    });
}

/**
 * Lookup identity by platform credentials
 */
async function lookupIdentity() {
    const token = localStorage.getItem('stark_token');
    const platform = document.getElementById('lookup-platform').value;
    const userId = document.getElementById('lookup-user-id').value.trim();

    if (!userId) {
        showError('Please enter a user ID');
        return;
    }

    const resultDiv = document.getElementById('identity-result');
    const contentDiv = document.getElementById('identity-content');

    resultDiv.classList.remove('hidden');
    contentDiv.innerHTML = '<div class="text-slate-400">Looking up identity...</div>';

    try {
        const response = await fetch(`${API_BASE}/identity?channel_type=${platform}&platform_user_id=${encodeURIComponent(userId)}`, {
            headers: { 'Authorization': `Bearer ${token}` }
        });

        if (response.ok) {
            const data = await response.json();
            renderIdentity(data);
        } else if (response.status === 404) {
            contentDiv.innerHTML = `
                <div class="text-slate-500 text-center py-4">
                    <p>No identity found for this user.</p>
                    <p class="text-sm mt-2">An identity will be created when they first interact with the bot.</p>
                </div>
            `;
        } else {
            contentDiv.innerHTML = '<div class="text-red-400">Failed to lookup identity.</div>';
        }
    } catch (error) {
        console.error('Failed to lookup identity:', error);
        contentDiv.innerHTML = '<div class="text-red-400">Failed to lookup identity.</div>';
    }
}

/**
 * Render identity details
 */
function renderIdentity(identity) {
    const contentDiv = document.getElementById('identity-content');

    const linkedAccounts = identity.linked_accounts || [];

    contentDiv.innerHTML = `
        <div class="space-y-4">
            <div class="flex items-center gap-4">
                <div class="w-12 h-12 rounded-full bg-stark-500/20 flex items-center justify-center">
                    <svg class="w-6 h-6 text-stark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path>
                    </svg>
                </div>
                <div>
                    <div class="text-white font-medium">Identity ID</div>
                    <div class="text-stark-400 font-mono text-sm">${escapeHtml(identity.identity_id)}</div>
                </div>
            </div>

            <div class="text-xs text-slate-500">
                Created: ${formatDate(identity.created_at)}
            </div>

            <div class="border-t border-slate-700 pt-4">
                <h4 class="text-sm font-medium text-slate-300 mb-3">Linked Accounts (${linkedAccounts.length})</h4>
                ${linkedAccounts.length > 0 ? `
                    <div class="space-y-2">
                        ${linkedAccounts.map(account => `
                            <div class="flex items-center justify-between p-3 bg-slate-900 rounded-lg">
                                <div class="flex items-center gap-3">
                                    <div class="w-8 h-8 rounded-lg flex items-center justify-center ${account.channel_type === 'telegram' ? 'bg-blue-500/20 text-blue-400' : 'bg-purple-500/20 text-purple-400'}">
                                        ${getChannelIcon(account.channel_type)}
                                    </div>
                                    <div>
                                        <div class="text-white text-sm">${escapeHtml(account.platform_user_name || 'Unknown')}</div>
                                        <div class="text-slate-500 text-xs">${capitalize(account.channel_type)} - ${escapeHtml(account.platform_user_id)}</div>
                                    </div>
                                </div>
                                <div class="flex items-center gap-2">
                                    ${account.is_verified ? `
                                        <span class="px-2 py-1 bg-green-500/20 text-green-400 text-xs rounded">Verified</span>
                                    ` : `
                                        <span class="px-2 py-1 bg-slate-700 text-slate-400 text-xs rounded">Unverified</span>
                                    `}
                                </div>
                            </div>
                        `).join('')}
                    </div>
                ` : `
                    <div class="text-slate-500 text-sm">No linked accounts.</div>
                `}
            </div>
        </div>
    `;

    // Pre-fill link form with this identity ID
    document.getElementById('link-identity-id').value = identity.identity_id;
}

/**
 * Link a new platform account to an identity
 */
async function linkIdentity() {
    const token = localStorage.getItem('stark_token');
    const identityId = document.getElementById('link-identity-id').value.trim();
    const platform = document.getElementById('link-platform').value;
    const userId = document.getElementById('link-user-id').value.trim();

    if (!identityId) {
        showError('Please enter an identity ID');
        return;
    }

    if (!userId) {
        showError('Please enter a user ID');
        return;
    }

    try {
        const response = await fetch(`${API_BASE}/identity/link`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${token}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                identity_id: identityId,
                channel_type: platform,
                platform_user_id: userId
            })
        });

        if (response.ok) {
            const data = await response.json();
            showSuccess('Identity linked successfully');
            renderIdentity(data);
            document.getElementById('link-user-id').value = '';
        } else if (response.status === 409) {
            showError('This platform user is already linked to an identity');
        } else {
            const data = await response.json();
            showError(data.error || 'Failed to link identity');
        }
    } catch (error) {
        console.error('Failed to link identity:', error);
        showError('Failed to link identity');
    }
}

/**
 * Get channel type icon SVG
 */
function getChannelIcon(type) {
    if (type === 'telegram') {
        return `<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
            <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
        </svg>`;
    } else {
        return `<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
            <path d="M5.042 15.165a2.528 2.528 0 0 1-2.52 2.523A2.528 2.528 0 0 1 0 15.165a2.527 2.527 0 0 1 2.522-2.52h2.52v2.52zM6.313 15.165a2.527 2.527 0 0 1 2.521-2.52 2.527 2.527 0 0 1 2.521 2.52v6.313A2.528 2.528 0 0 1 8.834 24a2.528 2.528 0 0 1-2.521-2.522v-6.313zM8.834 5.042a2.528 2.528 0 0 1-2.521-2.52A2.528 2.528 0 0 1 8.834 0a2.528 2.528 0 0 1 2.521 2.522v2.52H8.834zM8.834 6.313a2.528 2.528 0 0 1 2.521 2.521 2.528 2.528 0 0 1-2.521 2.521H2.522A2.528 2.528 0 0 1 0 8.834a2.528 2.528 0 0 1 2.522-2.521h6.312zM18.956 8.834a2.528 2.528 0 0 1 2.522-2.521A2.528 2.528 0 0 1 24 8.834a2.528 2.528 0 0 1-2.522 2.521h-2.522V8.834zM17.688 8.834a2.528 2.528 0 0 1-2.523 2.521 2.527 2.527 0 0 1-2.52-2.521V2.522A2.527 2.527 0 0 1 15.165 0a2.528 2.528 0 0 1 2.523 2.522v6.312zM15.165 18.956a2.528 2.528 0 0 1 2.523 2.522A2.528 2.528 0 0 1 15.165 24a2.527 2.527 0 0 1-2.52-2.522v-2.522h2.52zM15.165 17.688a2.527 2.527 0 0 1-2.52-2.523 2.526 2.526 0 0 1 2.52-2.52h6.313A2.527 2.527 0 0 1 24 15.165a2.528 2.528 0 0 1-2.522 2.523h-6.313z"/>
        </svg>`;
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

function capitalize(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
}

function formatDate(dateStr) {
    const date = new Date(dateStr);
    return date.toLocaleString();
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', init);
