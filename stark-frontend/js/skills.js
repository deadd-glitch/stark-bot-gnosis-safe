// Skills management page

const API_BASE = window.location.origin;

let skills = [];
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

// Source badge colors
const sourceColors = {
    bundled: 'bg-blue-500/20 text-blue-400 border-blue-500/50',
    managed: 'bg-purple-500/20 text-purple-400 border-purple-500/50',
    workspace: 'bg-green-500/20 text-green-400 border-green-500/50'
};

document.addEventListener('DOMContentLoaded', () => {
    authToken = getAuthToken();
    if (!authToken) return;

    loadSkills();
    setupEventListeners();
    setupUpload();
});

function setupEventListeners() {
    // Reload button
    document.getElementById('reload-skills').addEventListener('click', reloadSkills);

    // Close modal
    document.getElementById('close-modal').addEventListener('click', closeModal);
    document.getElementById('skill-modal').addEventListener('click', (e) => {
        if (e.target.id === 'skill-modal') closeModal();
    });

    // Escape key closes modal
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') closeModal();
    });

    // Logout
    document.getElementById('logout-btn').addEventListener('click', () => {
        window.location.href = '/index.html';
    });
}

function setupUpload() {
    const uploadZone = document.getElementById('upload-zone');
    const fileInput = document.getElementById('file-input');

    // Click to upload
    uploadZone.addEventListener('click', () => fileInput.click());

    // File input change
    fileInput.addEventListener('change', (e) => {
        if (e.target.files.length > 0) {
            uploadFile(e.target.files[0]);
        }
    });

    // Drag and drop
    uploadZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        uploadZone.classList.add('border-stark-500', 'bg-stark-500/10');
    });

    uploadZone.addEventListener('dragleave', (e) => {
        e.preventDefault();
        uploadZone.classList.remove('border-stark-500', 'bg-stark-500/10');
    });

    uploadZone.addEventListener('drop', (e) => {
        e.preventDefault();
        uploadZone.classList.remove('border-stark-500', 'bg-stark-500/10');

        if (e.dataTransfer.files.length > 0) {
            const file = e.dataTransfer.files[0];
            if (file.name.endsWith('.zip')) {
                uploadFile(file);
            } else {
                showError('Please upload a ZIP file');
            }
        }
    });
}

async function uploadFile(file) {
    const progressContainer = document.getElementById('upload-progress');
    const progressBar = document.getElementById('progress-bar');

    progressContainer.classList.remove('hidden');
    progressBar.style.width = '0%';

    try {
        const formData = new FormData();
        formData.append('file', file);

        // Simulate progress (real progress would require XHR)
        progressBar.style.width = '50%';

        const response = await authFetch(`${API_BASE}/api/skills/upload`, {
            method: 'POST',
            body: formData
        });

        progressBar.style.width = '100%';

        if (!response || !response.ok) {
            const data = await response?.json();
            throw new Error(data?.error || 'Failed to upload skill');
        }

        const result = await response.json();

        if (result.success) {
            showSuccess(`Skill "${result.skill?.name || 'unknown'}" uploaded successfully`);
            await loadSkills();
        } else {
            throw new Error(result.error || 'Upload failed');
        }
    } catch (error) {
        console.error('Error uploading skill:', error);
        showError(error.message || 'Failed to upload skill');
    } finally {
        setTimeout(() => {
            progressContainer.classList.add('hidden');
            progressBar.style.width = '0%';
            document.getElementById('file-input').value = '';
        }, 1000);
    }
}

async function loadSkills() {
    try {
        const response = await authFetch(`${API_BASE}/api/skills`);
        if (!response || !response.ok) throw new Error('Failed to load skills');

        skills = await response.json();
        renderSkills();
    } catch (error) {
        console.error('Error loading skills:', error);
        document.getElementById('skills-list').innerHTML =
            '<div class="text-red-400">Failed to load skills</div>';
    }
}

async function reloadSkills() {
    const btn = document.getElementById('reload-skills');
    btn.disabled = true;
    btn.innerHTML = `
        <svg class="w-4 h-4 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
        </svg>
        Reloading...
    `;

    try {
        const response = await authFetch(`${API_BASE}/api/skills/reload`, { method: 'POST' });
        if (!response || !response.ok) throw new Error('Failed to reload skills');

        const result = await response.json();
        showSuccess(`Reloaded skills (${result.count || 0} total)`);
        await loadSkills();
    } catch (error) {
        console.error('Error reloading skills:', error);
        showError('Failed to reload skills');
    } finally {
        btn.disabled = false;
        btn.innerHTML = `
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
            </svg>
            Reload
        `;
    }
}

function renderSkills() {
    const container = document.getElementById('skills-list');
    const emptyState = document.getElementById('empty-state');

    if (!skills.length) {
        container.innerHTML = '';
        emptyState.classList.remove('hidden');
        return;
    }

    emptyState.classList.add('hidden');

    const html = skills.map(skill => {
        const sourceClass = sourceColors[skill.source] || sourceColors.managed;
        const toolsHtml = skill.requires_tools?.length
            ? skill.requires_tools.map(t => `<span class="px-2 py-0.5 bg-slate-700 rounded text-xs">${t}</span>`).join('')
            : '<span class="text-slate-500 text-xs">None</span>';

        return `
            <div class="bg-slate-800 border border-slate-700 rounded-xl p-5 hover:border-slate-600 transition-colors">
                <div class="flex items-start justify-between mb-3">
                    <div class="flex items-center gap-3">
                        <h3 class="text-lg font-semibold text-white">${skill.name}</h3>
                        <span class="px-2 py-0.5 text-xs rounded border ${sourceClass}">${skill.source}</span>
                        ${skill.version ? `<span class="text-xs text-slate-500">v${skill.version}</span>` : ''}
                    </div>
                    <div class="flex items-center gap-2">
                        <label class="relative inline-flex items-center cursor-pointer">
                            <input type="checkbox" class="sr-only peer skill-toggle"
                                   data-skill="${skill.name}"
                                   ${skill.enabled !== false ? 'checked' : ''}>
                            <div class="w-9 h-5 bg-slate-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-stark-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-stark-500"></div>
                        </label>
                        <button class="view-skill text-slate-400 hover:text-stark-400 p-1" data-skill="${skill.name}" title="View details">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                            </svg>
                        </button>
                        <button class="delete-skill text-slate-400 hover:text-red-400 p-1" data-skill="${skill.name}" title="Delete skill">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                            </svg>
                        </button>
                    </div>
                </div>
                <p class="text-slate-400 text-sm mb-3">${skill.description || 'No description'}</p>
                <div class="flex items-center gap-2">
                    <span class="text-xs text-slate-500">Required tools:</span>
                    <div class="flex flex-wrap gap-1">${toolsHtml}</div>
                </div>
            </div>
        `;
    }).join('');

    container.innerHTML = html;

    // Add event listeners
    container.querySelectorAll('.skill-toggle').forEach(toggle => {
        toggle.addEventListener('change', (e) => toggleSkill(e.target.dataset.skill, e.target.checked));
    });

    container.querySelectorAll('.view-skill').forEach(btn => {
        btn.addEventListener('click', () => viewSkill(btn.dataset.skill));
    });

    container.querySelectorAll('.delete-skill').forEach(btn => {
        btn.addEventListener('click', () => deleteSkill(btn.dataset.skill));
    });
}

async function toggleSkill(skillName, enabled) {
    try {
        const response = await authFetch(`${API_BASE}/api/skills/${encodeURIComponent(skillName)}/enabled`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ enabled })
        });

        if (!response || !response.ok) throw new Error('Failed to update skill');

        // Update local state
        const skill = skills.find(s => s.name === skillName);
        if (skill) skill.enabled = enabled;

        showSuccess(`${skillName} ${enabled ? 'enabled' : 'disabled'}`);
    } catch (error) {
        console.error('Error toggling skill:', error);
        showError('Failed to update skill');
        // Revert UI
        loadSkills();
    }
}

async function viewSkill(skillName) {
    try {
        const response = await authFetch(`${API_BASE}/api/skills/${encodeURIComponent(skillName)}`);
        if (!response || !response.ok) throw new Error('Failed to load skill details');

        const data = await response.json();
        showSkillModal(data.skill);
    } catch (error) {
        console.error('Error loading skill details:', error);
        showError('Failed to load skill details');
    }
}

async function deleteSkill(skillName) {
    if (!confirm(`Are you sure you want to delete the skill "${skillName}"?`)) {
        return;
    }

    try {
        const response = await authFetch(`${API_BASE}/api/skills/${encodeURIComponent(skillName)}`, {
            method: 'DELETE'
        });

        if (!response || !response.ok) throw new Error('Failed to delete skill');

        showSuccess(`Skill "${skillName}" deleted`);
        await loadSkills();
    } catch (error) {
        console.error('Error deleting skill:', error);
        showError('Failed to delete skill');
    }
}

function showSkillModal(skill) {
    document.getElementById('modal-title').textContent = skill.name;

    const argsHtml = skill.arguments && skill.arguments.length
        ? skill.arguments.map(arg => `
            <tr class="border-b border-slate-700">
                <td class="py-2 text-stark-400 font-mono">${arg.name}</td>
                <td class="py-2 text-slate-400">${arg.description || '-'}</td>
                <td class="py-2 text-slate-500">${arg.default !== undefined && arg.default !== null ? arg.default : '-'}</td>
            </tr>
        `).join('')
        : '<tr><td colspan="3" class="py-2 text-slate-500">No arguments</td></tr>';

    const scriptsHtml = skill.scripts && skill.scripts.length
        ? `
            <div>
                <h4 class="text-sm font-medium text-slate-300 mb-2">Scripts</h4>
                <div class="flex flex-wrap gap-2">
                    ${skill.scripts.map(s => `
                        <span class="px-2 py-1 bg-amber-500/20 text-amber-400 rounded text-sm">
                            ${s.name} <span class="text-amber-600">(${s.language})</span>
                        </span>
                    `).join('')}
                </div>
            </div>
        `
        : '';

    const html = `
        <div class="space-y-6">
            <div>
                <h4 class="text-sm font-medium text-slate-300 mb-2">Description</h4>
                <p class="text-slate-400">${skill.description || 'No description'}</p>
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div>
                    <h4 class="text-sm font-medium text-slate-300 mb-2">Version</h4>
                    <p class="text-slate-400">${skill.version || 'N/A'}</p>
                </div>
                <div>
                    <h4 class="text-sm font-medium text-slate-300 mb-2">Source</h4>
                    <p class="text-slate-400 capitalize">${skill.source}</p>
                </div>
            </div>

            <div>
                <h4 class="text-sm font-medium text-slate-300 mb-2">Required Tools</h4>
                <div class="flex flex-wrap gap-2">
                    ${skill.requires_tools?.length
                        ? skill.requires_tools.map(t => `<span class="px-2 py-1 bg-slate-700 rounded text-sm">${t}</span>`).join('')
                        : '<span class="text-slate-500">None</span>'
                    }
                </div>
            </div>

            ${skill.requires_binaries?.length ? `
                <div>
                    <h4 class="text-sm font-medium text-slate-300 mb-2">Required Binaries</h4>
                    <div class="flex flex-wrap gap-2">
                        ${skill.requires_binaries.map(b => `<span class="px-2 py-1 bg-orange-500/20 text-orange-400 rounded text-sm">${b}</span>`).join('')}
                    </div>
                </div>
            ` : ''}

            ${scriptsHtml}

            <div>
                <h4 class="text-sm font-medium text-slate-300 mb-2">Arguments</h4>
                <table class="w-full text-sm">
                    <thead>
                        <tr class="text-left text-slate-500 border-b border-slate-700">
                            <th class="py-2">Name</th>
                            <th class="py-2">Description</th>
                            <th class="py-2">Default</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${argsHtml}
                    </tbody>
                </table>
            </div>

            ${skill.prompt_template ? `
                <div>
                    <h4 class="text-sm font-medium text-slate-300 mb-2">Prompt Template</h4>
                    <pre class="bg-slate-900 rounded p-4 text-xs text-slate-300 overflow-auto max-h-48">${escapeHtml(skill.prompt_template)}</pre>
                </div>
            ` : ''}
        </div>
    `;

    document.getElementById('modal-content').innerHTML = html;
    document.getElementById('skill-modal').classList.remove('hidden');
}

function closeModal() {
    document.getElementById('skill-modal').classList.add('hidden');
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
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
