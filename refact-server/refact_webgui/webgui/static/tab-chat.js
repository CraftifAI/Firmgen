import { general_error } from './error.js';

let chatMessages = [];
let chatId = 'webui-' + Date.now();
let currentAbortController = null;

export async function init() {
    try {
        let req = await fetch('/tab-chat.html');
        if (!req.ok) {
            throw new Error(`Failed to load chat HTML: ${req.status} ${req.statusText}`);
        }
        const html = await req.text();
        const chatElement = document.querySelector('#chat');
        if (!chatElement) {
            throw new Error('Chat element #chat not found in DOM');
        }
        chatElement.innerHTML = html;
        
        setupEventListeners();
        await checkConnection();
        await loadModels();
        await loadTools();
    } catch (error) {
        console.error('Failed to initialize chat tab:', error);
        const chatElement = document.querySelector('#chat');
        if (chatElement) {
            chatElement.innerHTML = `
                <div class="alert alert-danger">
                    <h5>Failed to load Chat tab</h5>
                    <p>Error: ${error.message}</p>
                    <p>Please check the browser console for more details.</p>
                </div>
            `;
        }
    }
}

function setupEventListeners() {
    document.getElementById('chat-send')?.addEventListener('click', sendMessage);
    document.getElementById('chat-input')?.addEventListener('keypress', (e) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    });
    document.getElementById('chat-clear')?.addEventListener('click', clearChat);
    document.getElementById('chat-check-connection')?.addEventListener('click', checkConnection);
}

async function checkConnection() {
    const statusEl = document.getElementById('chat-connection-status');
    statusEl.textContent = 'Checking...';
    statusEl.className = 'badge bg-secondary';
    
    try {
        const response = await fetch('/tab-chat-ping');
        const data = await response.json();
        
        if (data.success && data.connected) {
            if (data.auto_detected) {
                statusEl.textContent = `Connected (port ${data.port}, auto-detected)`;
            } else {
                statusEl.textContent = `Connected (port ${data.port})`;
            }
            statusEl.className = 'badge bg-success';
        } else {
            statusEl.textContent = 'Disconnected';
            statusEl.className = 'badge bg-danger';
            if (data.error) {
                showMessage('system', `Connection Error: ${data.error}`, true);
            }
        }
    } catch (error) {
        statusEl.textContent = 'Error';
        statusEl.className = 'badge bg-danger';
        console.error('Connection check error:', error);
    }
}

async function loadModels() {
    try {
        const response = await fetch('/tab-chat-caps');
        const data = await response.json();
        
        if (data.success && data.caps && data.caps.chat_models) {
            const modelSelect = document.getElementById('chat-model');
            modelSelect.innerHTML = '<option value="">Auto-select model</option>';
            
            Object.keys(data.caps.chat_models).forEach(modelName => {
                const option = document.createElement('option');
                option.value = modelName;
                option.textContent = modelName;
                modelSelect.appendChild(option);
            });
        }
    } catch (error) {
        console.error('Error loading models:', error);
    }
}

async function loadTools() {
    const toolsList = document.getElementById('chat-tools-list');
    
    try {
        const response = await fetch('/tab-chat-tools');
        const data = await response.json();
        
        if (data.success && data.tools) {
            if (data.tools.length === 0) {
                toolsList.innerHTML = '<p class="text-muted">No tools available</p>';
            } else {
                let html = '<div class="row">';
                data.tools.forEach(tool => {
                    if (tool.function && tool.function.name) {
                        const toolName = tool.function.name;
                        const toolDesc = tool.function.description || 'No description';
                        html += `
                            <div class="col-md-6 mb-2">
                                <div class="card">
                                    <div class="card-body p-2">
                                        <h6 class="card-title mb-1">${toolName}</h6>
                                        <p class="card-text small text-muted mb-0">${toolDesc}</p>
                                    </div>
                                </div>
                            </div>
                        `;
                    }
                });
                html += '</div>';
                toolsList.innerHTML = html;
            }
        } else {
            toolsList.innerHTML = `<p class="text-danger">Error loading tools: ${data.error || 'Unknown error'}</p>`;
        }
    } catch (error) {
        console.error('Error loading tools:', error);
        toolsList.innerHTML = '<p class="text-danger">Failed to load tools. Make sure the refact agent is running.</p>';
    }
}

function clearChat() {
    chatMessages = [];
    chatId = 'webui-' + Date.now();
    const messagesEl = document.getElementById('chat-messages');
    messagesEl.innerHTML = `
        <div class="text-center text-muted">
            <p>Start a conversation with the Refact Agent</p>
            <p><small>The agent has access to C2000 tools and can help with your embedded development tasks.</small></p>
        </div>
    `;
}

function showMessage(role, content, isError = false) {
    const messagesEl = document.getElementById('chat-messages');
    
    // Remove placeholder if exists
    const placeholder = messagesEl.querySelector('.text-center.text-muted');
    if (placeholder) {
        placeholder.remove();
    }
    
    const messageDiv = document.createElement('div');
    messageDiv.className = `mb-3 ${role === 'user' ? 'text-end' : ''}`;
    
    const bgClass = isError ? 'bg-danger text-white' : 
                   role === 'user' ? 'bg-primary text-white' : 
                   'bg-light';
    
    messageDiv.innerHTML = `
        <div class="d-inline-block p-3 rounded ${bgClass}" style="max-width: 80%;">
            <div class="small mb-1 ${role === 'user' ? 'text-white-50' : 'text-muted'}">
                ${role === 'user' ? 'You' : role === 'system' ? 'System' : 'Agent'}
            </div>
            <div class="${role === 'user' ? 'text-white' : ''}">${escapeHtml(content)}</div>
        </div>
    `;
    
    messagesEl.appendChild(messageDiv);
    messagesEl.scrollTop = messagesEl.scrollHeight;
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

async function sendMessage() {
    const inputEl = document.getElementById('chat-input');
    const message = inputEl.value.trim();
    
    if (!message) {
        return;
    }
    
    // Clear input
    inputEl.value = '';
    
    // Add user message to chat
    chatMessages.push({ role: 'user', content: message });
    showMessage('user', message);
    
    // Show thinking indicator
    const messagesEl = document.getElementById('chat-messages');
    const thinkingDiv = document.createElement('div');
    thinkingDiv.id = 'thinking-indicator';
    thinkingDiv.className = 'mb-3';
    thinkingDiv.innerHTML = `
        <div class="d-inline-block p-3 rounded bg-light">
            <div class="small text-muted mb-1">Agent</div>
            <div class="text-muted">
                <span class="spinner-border spinner-border-sm me-2" role="status"></span>
                Thinking...
            </div>
        </div>
    `;
    messagesEl.appendChild(thinkingDiv);
    messagesEl.scrollTop = messagesEl.scrollHeight;
    
    // Abort previous request if any
    if (currentAbortController) {
        currentAbortController.abort();
    }
    currentAbortController = new AbortController();
    
    try {
        const modelSelect = document.getElementById('chat-model');
        const streamCheckbox = document.getElementById('chat-stream');
        
        // Format messages properly for refact agent
        const formattedMessages = chatMessages.map(msg => ({
            role: msg.role,
            content: msg.content
        }));
        
        const requestBody = {
            messages: formattedMessages,
            model: modelSelect.value || null,
            stream: streamCheckbox.checked,
            chat_id: chatId
        };
        
        if (streamCheckbox.checked) {
            await sendStreamingMessage(requestBody);
        } else {
            await sendNonStreamingMessage(requestBody);
        }
    } catch (error) {
        if (error.name === 'AbortError') {
            return; // Request was aborted
        }
        console.error('Error sending message:', error);
        document.getElementById('thinking-indicator')?.remove();
        
        // Show more detailed error
        let errorMsg = error.message || 'Unknown error';
        if (errorMsg.includes('network error') || errorMsg.includes('Failed to fetch')) {
            errorMsg = `Network error: Could not connect to agent. Make sure the agent is running. Check connection status above.`;
        }
        showMessage('system', `Error: ${errorMsg}`, true);
        general_error('Failed to send message');
    }
}

async function sendStreamingMessage(requestBody) {
    const response = await fetch('/tab-chat-send', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestBody),
        signal: currentAbortController.signal
    });
    
    if (!response.ok) {
        const error = await response.json();
        throw new Error(error.error || 'Failed to send message');
    }
    
    // Remove thinking indicator
    document.getElementById('thinking-indicator')?.remove();
    
    // Create assistant message div
    const messagesEl = document.getElementById('chat-messages');
    const assistantDiv = document.createElement('div');
    assistantDiv.className = 'mb-3';
    assistantDiv.innerHTML = `
        <div class="d-inline-block p-3 rounded bg-light">
            <div class="small text-muted mb-1">Agent</div>
            <div id="assistant-message-content"></div>
        </div>
    `;
    messagesEl.appendChild(assistantDiv);
    const contentEl = assistantDiv.querySelector('#assistant-message-content');
    
    let fullContent = '';
    let buffer = '';  // Buffer for incomplete lines
    
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    
    try {
        while (true) {
            const { done, value } = await reader.read();
            
            if (done) {
                // Process any remaining buffer
                if (buffer.trim()) {
                    const lines = buffer.split('\n');
                    for (const line of lines) {
                        if (line.startsWith('data: ')) {
                            try {
                                const data = JSON.parse(line.slice(6));
                                if (data.choices && data.choices[0] && data.choices[0].delta) {
                                    const delta = data.choices[0].delta;
                                    if (delta.content) {
                                        fullContent += delta.content;
                                        contentEl.textContent = fullContent;
                                        messagesEl.scrollTop = messagesEl.scrollHeight;
                                    }
                                }
                            } catch (e) {
                                // Ignore parse errors
                            }
                        }
                    }
                }
                break;
            }
            
            // Decode chunk and add to buffer
            buffer += decoder.decode(value, { stream: true });
            
            // Process complete lines
            const lines = buffer.split('\n');
            // Keep the last (potentially incomplete) line in buffer
            buffer = lines.pop() || '';
            
            for (const line of lines) {
                if (line.startsWith('data: ')) {
                    try {
                        const data = JSON.parse(line.slice(6));
                        if (data.choices && data.choices[0] && data.choices[0].delta) {
                            const delta = data.choices[0].delta;
                            if (delta.content) {
                                fullContent += delta.content;
                                contentEl.textContent = fullContent;
                                messagesEl.scrollTop = messagesEl.scrollHeight;
                            }
                        }
                    } catch (e) {
                        // Ignore parse errors
                    }
                }
            }
        }
    } catch (error) {
        console.error('Error reading stream:', error);
        // Show partial content if available
        if (fullContent) {
            contentEl.textContent = fullContent + '\n\n[Stream interrupted]';
        } else {
            contentEl.textContent = '[Error: Stream interrupted. Please try again.]';
        }
        showMessage('system', `Stream error: ${error.message}`, true);
    }
    
    // Add to chat messages
    if (fullContent) {
        chatMessages.push({ role: 'assistant', content: fullContent });
    }
}

async function sendNonStreamingMessage(requestBody) {
    const response = await fetch('/tab-chat-send', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestBody),
        signal: currentAbortController.signal
    });
    
    if (!response.ok) {
        // Try to get error message
        let errorMsg = `HTTP ${response.status}`;
        try {
            const errorData = await response.json();
            errorMsg = errorData.error || errorMsg;
        } catch {
            errorMsg = await response.text() || errorMsg;
        }
        throw new Error(errorMsg);
    }
    
    const data = await response.json();
    
    // Remove thinking indicator
    document.getElementById('thinking-indicator')?.remove();
    
    if (data.success && data.response) {
        let assistantMessage = '';
        
        // The response format from refact agent might be different
        // Try different response structures
        if (data.response.choices && data.response.choices[0] && data.response.choices[0].message) {
            assistantMessage = data.response.choices[0].message.content || '';
        } else if (data.response.message && data.response.message.content) {
            assistantMessage = data.response.message.content;
        } else if (data.response.content) {
            assistantMessage = data.response.content;
        } else if (typeof data.response === 'string') {
            assistantMessage = data.response;
        } else if (data.response.messages && data.response.messages.length > 0) {
            // Last message might be the assistant response
            const lastMsg = data.response.messages[data.response.messages.length - 1];
            if (lastMsg.role === 'assistant' && lastMsg.content) {
                assistantMessage = typeof lastMsg.content === 'string' ? lastMsg.content : JSON.stringify(lastMsg.content);
            }
        }
        
        if (assistantMessage) {
            chatMessages.push({ role: 'assistant', content: assistantMessage });
            showMessage('assistant', assistantMessage);
        } else {
            // Show the full response for debugging
            console.log('Full response:', data.response);
            showMessage('system', `Received response but couldn't extract message. Check console for details.`, true);
        }
    } else {
        throw new Error(data.error || 'Failed to get response');
    }
}

export function tab_switched_here() {
    checkConnection();
    loadTools();
}

export function tab_switched_away() {
    // Abort any ongoing requests
    if (currentAbortController) {
        currentAbortController.abort();
        currentAbortController = null;
    }
}

export function tab_update_each_couple_of_seconds() {
    // Periodic updates if needed
}

