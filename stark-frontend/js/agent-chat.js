document.addEventListener('DOMContentLoaded', function() {
    const token = localStorage.getItem('stark_token');

    if (!token) {
        redirectToLogin();
        return;
    }

    // Validate token
    validateToken(token);

    // Handle logout
    document.getElementById('logout-btn').addEventListener('click', () => handleLogout(token));

    // Handle chat form
    document.getElementById('chat-form').addEventListener('submit', handleSendMessage);

    // Handle input for slash command autocomplete
    const input = document.getElementById('message-input');
    input.addEventListener('input', handleInputChange);
    input.addEventListener('keydown', handleInputKeydown);

    // Focus input
    input.focus();

    // Create autocomplete dropdown
    createAutocompleteDropdown();
});

// Conversation history
let conversationHistory = [];
let sessionStartTime = Date.now();
let messageCount = 0;
let autocompleteVisible = false;
let selectedAutocompleteIndex = 0;

// Slash commands definition
const slashCommands = {
    '/help': {
        description: 'Show available commands',
        usage: '/help',
        handler: cmdHelp
    },
    '/status': {
        description: 'Show session status and system info',
        usage: '/status',
        handler: cmdStatus
    },
    '/new': {
        description: 'Start a new conversation (clear history)',
        usage: '/new',
        aliases: ['/reset', '/clear'],
        handler: cmdNew
    },
    '/reset': {
        description: 'Start a new conversation (alias for /new)',
        usage: '/reset',
        hidden: true,
        handler: cmdNew
    },
    '/clear': {
        description: 'Clear chat display (alias for /new)',
        usage: '/clear',
        hidden: true,
        handler: cmdNew
    },
    '/skills': {
        description: 'List available skills',
        usage: '/skills',
        handler: cmdSkills
    },
    '/tools': {
        description: 'List available tools',
        usage: '/tools',
        handler: cmdTools
    },
    '/model': {
        description: 'Show current AI model configuration',
        usage: '/model',
        handler: cmdModel
    },
    '/compact': {
        description: 'Compact conversation context (summarize)',
        usage: '/compact',
        handler: cmdCompact
    },
    '/export': {
        description: 'Export conversation as JSON',
        usage: '/export',
        handler: cmdExport
    },
    '/debug': {
        description: 'Toggle debug mode for verbose output',
        usage: '/debug [on|off]',
        handler: cmdDebug
    },
    '/whoami': {
        description: 'Show current user info',
        usage: '/whoami',
        handler: cmdWhoami
    }
};

let debugMode = false;

function redirectToLogin() {
    window.location.href = '/';
}

async function validateToken(token) {
    try {
        const response = await fetch('/api/auth/validate', {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${token}`
            }
        });

        const data = await response.json();
        if (!data.valid) {
            localStorage.removeItem('stark_token');
            redirectToLogin();
        }
    } catch (error) {
        console.error('Validation error:', error);
    }
}

async function handleLogout(token) {
    try {
        await fetch('/api/auth/logout', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ token: token })
        });
    } catch (error) {
        console.error('Logout error:', error);
    } finally {
        localStorage.removeItem('stark_token');
        redirectToLogin();
    }
}

// Create autocomplete dropdown element
function createAutocompleteDropdown() {
    const dropdown = document.createElement('div');
    dropdown.id = 'command-autocomplete';
    dropdown.className = 'hidden absolute bottom-full left-0 right-0 mb-2 bg-slate-800 border border-slate-600 rounded-lg shadow-xl max-h-64 overflow-auto';

    const inputContainer = document.getElementById('message-input').parentElement;
    inputContainer.style.position = 'relative';
    inputContainer.appendChild(dropdown);
}

// Handle input changes for autocomplete
function handleInputChange(event) {
    const value = event.target.value;

    if (value.startsWith('/')) {
        showAutocomplete(value);
    } else {
        hideAutocomplete();
    }
}

// Handle keyboard navigation in autocomplete
function handleInputKeydown(event) {
    const dropdown = document.getElementById('command-autocomplete');

    if (!autocompleteVisible) return;

    const items = dropdown.querySelectorAll('.autocomplete-item');

    if (event.key === 'ArrowDown') {
        event.preventDefault();
        selectedAutocompleteIndex = Math.min(selectedAutocompleteIndex + 1, items.length - 1);
        updateAutocompleteSelection(items);
    } else if (event.key === 'ArrowUp') {
        event.preventDefault();
        selectedAutocompleteIndex = Math.max(selectedAutocompleteIndex - 1, 0);
        updateAutocompleteSelection(items);
    } else if (event.key === 'Tab' || event.key === 'Enter') {
        if (items.length > 0 && autocompleteVisible) {
            const selectedItem = items[selectedAutocompleteIndex];
            if (selectedItem && event.key === 'Tab') {
                event.preventDefault();
                const command = selectedItem.dataset.command;
                document.getElementById('message-input').value = command + ' ';
                hideAutocomplete();
            }
        }
    } else if (event.key === 'Escape') {
        hideAutocomplete();
    }
}

function updateAutocompleteSelection(items) {
    items.forEach((item, index) => {
        if (index === selectedAutocompleteIndex) {
            item.classList.add('bg-slate-700');
        } else {
            item.classList.remove('bg-slate-700');
        }
    });
}

function showAutocomplete(query) {
    const dropdown = document.getElementById('command-autocomplete');
    const searchTerm = query.toLowerCase();

    // Filter commands that match
    const matches = Object.entries(slashCommands)
        .filter(([cmd, info]) => !info.hidden && cmd.startsWith(searchTerm))
        .slice(0, 8);

    if (matches.length === 0) {
        hideAutocomplete();
        return;
    }

    dropdown.innerHTML = matches.map(([cmd, info], index) => `
        <div class="autocomplete-item px-4 py-2 cursor-pointer hover:bg-slate-700 ${index === 0 ? 'bg-slate-700' : ''}"
             data-command="${cmd}"
             onclick="selectCommand('${cmd}')">
            <div class="flex items-center justify-between">
                <span class="text-stark-400 font-medium">${cmd}</span>
                <span class="text-slate-500 text-xs">${info.usage}</span>
            </div>
            <p class="text-slate-400 text-sm">${info.description}</p>
        </div>
    `).join('');

    dropdown.classList.remove('hidden');
    autocompleteVisible = true;
    selectedAutocompleteIndex = 0;
}

function hideAutocomplete() {
    const dropdown = document.getElementById('command-autocomplete');
    dropdown.classList.add('hidden');
    autocompleteVisible = false;
    selectedAutocompleteIndex = 0;
}

function selectCommand(command) {
    document.getElementById('message-input').value = command + ' ';
    document.getElementById('message-input').focus();
    hideAutocomplete();
}

// Make selectCommand available globally
window.selectCommand = selectCommand;

async function handleSendMessage(event) {
    event.preventDefault();

    const token = localStorage.getItem('stark_token');
    const input = document.getElementById('message-input');
    const sendBtn = document.getElementById('send-btn');
    const message = input.value.trim();

    if (!message) return;

    hideAutocomplete();

    // Check if it's a slash command
    if (message.startsWith('/')) {
        handleSlashCommand(message);
        input.value = '';
        return;
    }

    // Add user message to UI and history
    addMessage(message, 'user');
    conversationHistory.push({ role: 'user', content: message });
    messageCount++;
    input.value = '';

    // Disable input while processing
    input.disabled = true;
    sendBtn.disabled = true;

    // Show typing indicator
    showTypingIndicator();

    try {
        const response = await fetch('/api/chat', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token}`
            },
            body: JSON.stringify({ messages: conversationHistory })
        });

        hideTypingIndicator();

        const data = await response.json();

        if (data.success && data.message) {
            addMessage(data.message.content, 'assistant');
            conversationHistory.push({ role: 'assistant', content: data.message.content });
            messageCount++;

            if (debugMode) {
                console.log('Response data:', data);
            }
        } else {
            const errorMsg = data.error || 'Failed to get response from AI';
            addMessage(`Error: ${errorMsg}`, 'error');
        }
    } catch (error) {
        hideTypingIndicator();
        console.error('Chat error:', error);
        addMessage('Error: Failed to connect to the server. Please try again.', 'error');
    } finally {
        // Re-enable input
        input.disabled = false;
        sendBtn.disabled = false;
        input.focus();
    }
}

// Slash command handler
function handleSlashCommand(input) {
    const parts = input.split(/\s+/);
    const command = parts[0].toLowerCase();
    const args = parts.slice(1);

    // Find the command
    const cmdInfo = slashCommands[command];

    if (cmdInfo && cmdInfo.handler) {
        // Show command in chat
        addMessage(input, 'command');
        // Execute handler
        cmdInfo.handler(args);
    } else {
        addMessage(input, 'command');
        addMessage(`Unknown command: ${command}. Type /help for available commands.`, 'system');
    }
}

// Command handlers
function cmdHelp(args) {
    const helpText = `**Available Commands**

${Object.entries(slashCommands)
    .filter(([_, info]) => !info.hidden)
    .map(([cmd, info]) => `\`${info.usage}\` — ${info.description}`)
    .join('\n')}

**Tips:**
• Commands are case-insensitive
• Use Tab to autocomplete commands
• Type / to see command suggestions`;

    addMessage(helpText, 'system');
}

async function cmdStatus(args) {
    const uptime = formatDuration(Date.now() - sessionStartTime);
    const token = localStorage.getItem('stark_token');

    let providerInfo = 'Unknown';
    try {
        const response = await fetch('/api/agent-settings', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();
        if (data.provider) {
            providerInfo = `${data.provider}${data.model ? ' / ' + data.model : ''}`;
        }
    } catch (e) {
        console.error('Failed to fetch provider info:', e);
    }

    const statusText = `**Session Status**

• **Messages:** ${messageCount} (${conversationHistory.length} in context)
• **Session Duration:** ${uptime}
• **Provider:** ${providerInfo}
• **Debug Mode:** ${debugMode ? 'ON' : 'OFF'}
• **Context Size:** ~${estimateTokens(conversationHistory)} tokens (est.)`;

    addMessage(statusText, 'system');
}

function cmdNew(args) {
    conversationHistory = [];
    messageCount = 0;
    sessionStartTime = Date.now();

    // Clear chat display
    const container = document.getElementById('messages-container');
    container.innerHTML = `
        <div class="flex gap-4 message-appear">
            <div class="w-8 h-8 bg-gradient-to-br from-stark-400 to-stark-600 rounded-full flex-shrink-0 flex items-center justify-center">
                <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                </svg>
            </div>
            <div class="flex-1 max-w-2xl">
                <div class="bg-slate-800 border border-slate-700 rounded-2xl rounded-tl-sm px-4 py-3">
                    <p class="text-slate-200">Conversation cleared. Starting fresh! How can I help you?</p>
                </div>
                <p class="text-xs text-slate-500 mt-1 ml-2">Just now</p>
            </div>
        </div>
    `;
}

async function cmdSkills(args) {
    const token = localStorage.getItem('stark_token');

    try {
        const response = await fetch('/api/skills', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        if (data.skills && data.skills.length > 0) {
            const skillsList = data.skills.map(s =>
                `• **${s.name}** — ${s.description || 'No description'}`
            ).join('\n');
            addMessage(`**Available Skills (${data.skills.length})**\n\n${skillsList}`, 'system');
        } else {
            addMessage('No skills are currently loaded. Add skills in the Skills page.', 'system');
        }
    } catch (e) {
        addMessage('Failed to fetch skills. Check the Skills page for configuration.', 'system');
    }
}

async function cmdTools(args) {
    const token = localStorage.getItem('stark_token');

    try {
        const response = await fetch('/api/tools', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        if (data.tools && data.tools.length > 0) {
            const enabledTools = data.tools.filter(t => t.enabled);
            const toolsList = data.tools.map(t =>
                `• ${t.enabled ? '✓' : '○'} **${t.name}** — ${t.description || 'No description'}`
            ).join('\n');
            addMessage(`**Available Tools (${enabledTools.length}/${data.tools.length} enabled)**\n\n${toolsList}`, 'system');
        } else {
            addMessage('No tools are configured. Check the Tools page.', 'system');
        }
    } catch (e) {
        addMessage('Failed to fetch tools. Check the Tools page for configuration.', 'system');
    }
}

async function cmdModel(args) {
    const token = localStorage.getItem('stark_token');

    try {
        const response = await fetch('/api/agent-settings', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        const modelInfo = `**Current Model Configuration**

• **Provider:** ${data.provider || 'Not configured'}
• **Model:** ${data.model || 'Default'}
• **Endpoint:** ${data.endpoint ? data.endpoint.substring(0, 50) + '...' : 'Default'}
• **Status:** ${data.provider ? 'Configured' : 'Not configured'}

To change the model, visit the Agent Settings page.`;

        addMessage(modelInfo, 'system');
    } catch (e) {
        addMessage('Failed to fetch model info. Check Agent Settings page.', 'system');
    }
}

function cmdCompact(args) {
    if (conversationHistory.length < 4) {
        addMessage('Not enough conversation history to compact. Keep chatting!', 'system');
        return;
    }

    // Create a summary request
    const summaryPrompt = `Please provide a brief summary of our conversation so far in 2-3 sentences, capturing the key topics and any important context.`;

    addMessage('Compacting conversation context... This will summarize the conversation and reduce token usage.', 'system');

    // For now, just inform the user - actual compaction would require backend support
    addMessage(`Current context: ${conversationHistory.length} messages (~${estimateTokens(conversationHistory)} tokens). Compaction would reduce this to a summary. Feature coming soon!`, 'system');
}

function cmdExport(args) {
    const exportData = {
        exported_at: new Date().toISOString(),
        session_duration_ms: Date.now() - sessionStartTime,
        message_count: messageCount,
        messages: conversationHistory
    };

    const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `starkbot-conversation-${new Date().toISOString().split('T')[0]}.json`;
    a.click();
    URL.revokeObjectURL(url);

    addMessage(`Conversation exported (${conversationHistory.length} messages).`, 'system');
}

function cmdDebug(args) {
    if (args.length > 0) {
        const mode = args[0].toLowerCase();
        debugMode = mode === 'on' || mode === 'true' || mode === '1';
    } else {
        debugMode = !debugMode;
    }

    addMessage(`Debug mode: **${debugMode ? 'ON' : 'OFF'}**\n\nWhen enabled, additional information will be logged to the browser console.`, 'system');
}

async function cmdWhoami(args) {
    const token = localStorage.getItem('stark_token');

    try {
        const response = await fetch('/api/dashboard', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        const data = await response.json();

        addMessage(`**Current User**\n\n• **Session:** Active\n• **Token:** ${token.substring(0, 8)}...`, 'system');
    } catch (e) {
        addMessage('Failed to fetch user info.', 'system');
    }
}

// Utility functions
function formatDuration(ms) {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
        return `${hours}h ${minutes % 60}m`;
    } else if (minutes > 0) {
        return `${minutes}m ${seconds % 60}s`;
    } else {
        return `${seconds}s`;
    }
}

function estimateTokens(messages) {
    // Rough estimate: ~4 characters per token
    const totalChars = messages.reduce((sum, m) => sum + (m.content || '').length, 0);
    return Math.round(totalChars / 4);
}

function addMessage(content, role) {
    const container = document.getElementById('messages-container');
    const messageDiv = document.createElement('div');
    messageDiv.className = 'flex gap-4 message-appear';

    const time = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

    if (role === 'user') {
        messageDiv.innerHTML = `
            <div class="flex-1"></div>
            <div class="max-w-2xl">
                <div class="bg-stark-500 rounded-2xl rounded-tr-sm px-4 py-3">
                    <p class="text-white whitespace-pre-wrap">${escapeHtml(content)}</p>
                </div>
                <p class="text-xs text-slate-500 mt-1 mr-2 text-right">${time}</p>
            </div>
            <div class="w-8 h-8 bg-slate-600 rounded-full flex-shrink-0 flex items-center justify-center">
                <svg class="w-4 h-4 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path>
                </svg>
            </div>
        `;
    } else if (role === 'command') {
        messageDiv.innerHTML = `
            <div class="flex-1"></div>
            <div class="max-w-2xl">
                <div class="bg-slate-700 border border-slate-600 rounded-2xl rounded-tr-sm px-4 py-2">
                    <p class="text-stark-400 font-mono text-sm">${escapeHtml(content)}</p>
                </div>
                <p class="text-xs text-slate-500 mt-1 mr-2 text-right">${time}</p>
            </div>
            <div class="w-8 h-8 bg-slate-700 rounded-full flex-shrink-0 flex items-center justify-center">
                <svg class="w-4 h-4 text-stark-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                </svg>
            </div>
        `;
    } else if (role === 'system') {
        // Parse simple markdown for system messages
        const formattedContent = formatSystemMessage(content);
        messageDiv.innerHTML = `
            <div class="w-8 h-8 bg-slate-700 rounded-full flex-shrink-0 flex items-center justify-center">
                <svg class="w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                </svg>
            </div>
            <div class="flex-1 max-w-2xl">
                <div class="bg-slate-800/50 border border-slate-700 rounded-2xl rounded-tl-sm px-4 py-3">
                    <div class="text-slate-300 text-sm whitespace-pre-wrap">${formattedContent}</div>
                </div>
                <p class="text-xs text-slate-500 mt-1 ml-2">${time}</p>
            </div>
        `;
    } else if (role === 'error') {
        messageDiv.innerHTML = `
            <div class="w-8 h-8 bg-red-500/20 rounded-full flex-shrink-0 flex items-center justify-center">
                <svg class="w-4 h-4 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                </svg>
            </div>
            <div class="flex-1 max-w-2xl">
                <div class="bg-red-500/20 border border-red-500/30 rounded-2xl rounded-tl-sm px-4 py-3">
                    <p class="text-red-400 whitespace-pre-wrap">${escapeHtml(content)}</p>
                </div>
                <p class="text-xs text-slate-500 mt-1 ml-2">${time}</p>
            </div>
        `;
    } else {
        messageDiv.innerHTML = `
            <div class="w-8 h-8 bg-gradient-to-br from-stark-400 to-stark-600 rounded-full flex-shrink-0 flex items-center justify-center">
                <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                </svg>
            </div>
            <div class="flex-1 max-w-2xl">
                <div class="bg-slate-800 border border-slate-700 rounded-2xl rounded-tl-sm px-4 py-3">
                    <p class="text-slate-200 whitespace-pre-wrap">${escapeHtml(content)}</p>
                </div>
                <p class="text-xs text-slate-500 mt-1 ml-2">${time}</p>
            </div>
        `;
    }

    container.appendChild(messageDiv);
    container.scrollTop = container.scrollHeight;
}

function formatSystemMessage(content) {
    // Simple markdown parsing for system messages
    let formatted = escapeHtml(content);

    // Bold: **text**
    formatted = formatted.replace(/\*\*([^*]+)\*\*/g, '<strong class="text-white">$1</strong>');

    // Code: `text`
    formatted = formatted.replace(/`([^`]+)`/g, '<code class="bg-slate-900 px-1 py-0.5 rounded text-stark-400">$1</code>');

    // Bullet points: •
    formatted = formatted.replace(/^• /gm, '<span class="text-stark-400">•</span> ');

    return formatted;
}

function showTypingIndicator() {
    document.getElementById('typing-indicator').classList.remove('hidden');
    document.getElementById('messages-container').scrollTop = document.getElementById('messages-container').scrollHeight;
}

function hideTypingIndicator() {
    document.getElementById('typing-indicator').classList.add('hidden');
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
