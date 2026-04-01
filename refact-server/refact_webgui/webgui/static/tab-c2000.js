import { general_error } from './error.js';

let config_data = null;
let examples_data = [];
let projects_data = [];

export async function init() {
    let req = await fetch('/tab-c2000.html');
    document.querySelector('#c2000').innerHTML = await req.text();
    
    // Setup event listeners
    setupEventListeners();
    
    // Load initial data
    await loadConfig();
    await loadProjects();
    await loadExamples();
}

function setupEventListeners() {
    // Configuration
    document.getElementById('c2000-config-save')?.addEventListener('click', saveConfig);
    document.getElementById('c2000-config-validate')?.addEventListener('click', validateConfig);
    
    // Projects
    document.getElementById('c2000-projects-refresh')?.addEventListener('click', loadProjects);
    
    // Examples
    document.getElementById('c2000-examples-refresh')?.addEventListener('click', loadExamples);
    document.getElementById('c2000-examples-search')?.addEventListener('input', filterExamples);
    
    // Target detection
    document.getElementById('c2000-target-detect-btn')?.addEventListener('click', detectTargets);
    
    // Debugging
    document.getElementById('c2000-debug-run')?.addEventListener('click', runDebugAnalysis);
    
    // Log analysis
    document.getElementById('c2000-logs-analyze-btn')?.addEventListener('click', analyzeLogs);
}

async function loadConfig() {
    try {
        const response = await fetch('/tab-c2000-config-get');
        const data = await response.json();
        
        if (data.success && data.config) {
            config_data = data.config;
            document.getElementById('c2000-ccs-path').value = data.config.ccs_path || '';
            document.getElementById('c2000-c2000ware-path').value = data.config.c2000ware_path || '';
            document.getElementById('c2000-workspace-path').value = data.config.workspace_path || '';
            document.getElementById('c2000-target-device').value = data.config.target_device || 'F28P65x';
            document.getElementById('c2000-debug-probe').value = data.config.debug_probe || 'XDS110';
        }
    } catch (error) {
        console.error('Error loading config:', error);
        general_error('Failed to load configuration');
    }
}

async function saveConfig() {
    const statusEl = document.getElementById('c2000-config-status');
    statusEl.innerHTML = '<div class="alert alert-info">Saving...</div>';
    
    try {
        const config = {
            ccs_path: document.getElementById('c2000-ccs-path').value,
            c2000ware_path: document.getElementById('c2000-c2000ware-path').value,
            workspace_path: document.getElementById('c2000-workspace-path').value,
            target_device: document.getElementById('c2000-target-device').value,
            debug_probe: document.getElementById('c2000-debug-probe').value
        };
        
        const response = await fetch('/tab-c2000-config-set', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(config)
        });
        
        const data = await response.json();
        
        if (data.success) {
            statusEl.innerHTML = '<div class="alert alert-success">Configuration saved successfully</div>';
            setTimeout(() => { statusEl.innerHTML = ''; }, 3000);
        } else {
            statusEl.innerHTML = `<div class="alert alert-danger">Error: ${data.error || 'Failed to save'}</div>`;
        }
    } catch (error) {
        console.error('Error saving config:', error);
        statusEl.innerHTML = '<div class="alert alert-danger">Failed to save configuration</div>';
        general_error('Failed to save configuration');
    }
}

async function validateConfig() {
    const statusEl = document.getElementById('c2000-config-status');
    statusEl.innerHTML = '<div class="alert alert-info">Validating...</div>';
    
    try {
        const response = await fetch('/tab-c2000-config-validate');
        const data = await response.json();
        
        if (data.success) {
            let html = '';
            if (data.valid) {
                html = '<div class="alert alert-success">Configuration is valid</div>';
            } else {
                html = '<div class="alert alert-warning">Configuration has issues:</div><ul>';
                data.errors.forEach(err => {
                    html += `<li>${err}</li>`;
                });
                html += '</ul>';
            }
            
            if (data.warnings && data.warnings.length > 0) {
                html += '<div class="alert alert-info">Warnings:</div><ul>';
                data.warnings.forEach(warn => {
                    html += `<li>${warn}</li>`;
                });
                html += '</ul>';
            }
            
            statusEl.innerHTML = html;
        } else {
            statusEl.innerHTML = `<div class="alert alert-danger">Error: ${data.error || 'Validation failed'}</div>`;
        }
    } catch (error) {
        console.error('Error validating config:', error);
        statusEl.innerHTML = '<div class="alert alert-danger">Failed to validate configuration</div>';
        general_error('Failed to validate configuration');
    }
}

async function loadProjects() {
    const tbody = document.getElementById('c2000-projects-tbody');
    tbody.innerHTML = '<tr><td colspan="3" class="text-center">Loading...</td></tr>';
    
    try {
        const response = await fetch('/tab-c2000-projects-list');
        const data = await response.json();
        
        if (data.success) {
            projects_data = data.projects || [];
            
            if (projects_data.length === 0) {
                tbody.innerHTML = '<tr><td colspan="3" class="text-center">No projects found</td></tr>';
            } else {
                tbody.innerHTML = projects_data.map(project => `
                    <tr>
                        <td>${project.name}</td>
                        <td><code>${project.path}</code></td>
                        <td>
                            <button class="btn btn-sm btn-primary me-1" onclick="buildProject('${project.name}')">Build</button>
                            <button class="btn btn-sm btn-success" onclick="flashProject('${project.name}')">Flash</button>
                        </td>
                    </tr>
                `).join('');
            }
        } else {
            tbody.innerHTML = '<tr><td colspan="3" class="text-center text-danger">Error loading projects</td></tr>';
        }
    } catch (error) {
        console.error('Error loading projects:', error);
        tbody.innerHTML = '<tr><td colspan="3" class="text-center text-danger">Failed to load projects</td></tr>';
        general_error('Failed to load projects');
    }
}

async function loadExamples() {
    const tbody = document.getElementById('c2000-examples-tbody');
    tbody.innerHTML = '<tr><td colspan="3" class="text-center">Loading...</td></tr>';
    
    try {
        const response = await fetch('/tab-c2000-examples-list');
        const data = await response.json();
        
        if (data.success) {
            examples_data = data.examples || [];
            renderExamples();
        } else {
            tbody.innerHTML = '<tr><td colspan="3" class="text-center text-danger">Error loading examples</td></tr>';
        }
    } catch (error) {
        console.error('Error loading examples:', error);
        tbody.innerHTML = '<tr><td colspan="3" class="text-center text-danger">Failed to load examples</td></tr>';
        general_error('Failed to load examples');
    }
}

function filterExamples() {
    const searchTerm = document.getElementById('c2000-examples-search').value.toLowerCase();
    renderExamples(searchTerm);
}

function renderExamples(searchTerm = '') {
    const tbody = document.getElementById('c2000-examples-tbody');
    
    let filtered = examples_data;
    if (searchTerm) {
        filtered = examples_data.filter(ex => 
            ex.name.toLowerCase().includes(searchTerm) || 
            ex.relative_path.toLowerCase().includes(searchTerm)
        );
    }
    
    if (filtered.length === 0) {
        tbody.innerHTML = '<tr><td colspan="3" class="text-center">No examples found</td></tr>';
    } else {
        tbody.innerHTML = filtered.map(example => `
            <tr>
                <td>${example.name}</td>
                <td><code>${example.relative_path}</code></td>
                <td>
                    <button class="btn btn-sm btn-primary" onclick="createProjectFromExample('${example.name}', '${example.path}')">Create Project</button>
                </td>
            </tr>
        `).join('');
    }
}

async function detectTargets() {
    const targetsList = document.getElementById('c2000-targets-list');
    targetsList.innerHTML = '<div class="alert alert-info">Detecting targets...</div>';
    
    try {
        const response = await fetch('/tab-c2000-target-detect');
        const data = await response.json();
        
        if (data.success && data.targets) {
            if (data.targets.length === 0) {
                targetsList.innerHTML = '<div class="alert alert-warning">No targets detected</div>';
            } else {
                targetsList.innerHTML = data.targets.map(target => `
                    <div class="card mb-2">
                        <div class="card-body">
                            <h6 class="card-title">${target.name}</h6>
                            <p class="card-text mb-1">
                                <strong>Device:</strong> ${target.device}<br>
                                <strong>Probe:</strong> ${target.probe}<br>
                                <strong>Status:</strong> 
                                <span class="badge ${target.connected ? 'bg-success' : 'bg-danger'}">
                                    ${target.connected ? 'Connected' : 'Disconnected'}
                                </span>
                            </p>
                        </div>
                    </div>
                `).join('');
            }
        } else {
            targetsList.innerHTML = '<div class="alert alert-danger">Failed to detect targets</div>';
        }
    } catch (error) {
        console.error('Error detecting targets:', error);
        targetsList.innerHTML = '<div class="alert alert-danger">Error detecting targets</div>';
        general_error('Failed to detect targets');
    }
}

async function runDebugAnalysis() {
    const resultsEl = document.getElementById('c2000-debug-results');
    resultsEl.innerHTML = '<div class="alert alert-info">Running debug analysis...</div>';
    
    try {
        const projectName = document.getElementById('c2000-debug-project-name').value;
        const issue = document.getElementById('c2000-debug-issue').value;
        const logFiles = document.getElementById('c2000-debug-log-files').value.split(',').map(f => f.trim()).filter(f => f);
        
        if (!projectName || !issue || logFiles.length === 0) {
            resultsEl.innerHTML = '<div class="alert alert-warning">Please fill in all fields</div>';
            return;
        }
        
        const response = await fetch('/tab-c2000-debug', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                project_name: projectName,
                issue_description: issue,
                log_files: logFiles
            })
        });
        
        const data = await response.json();
        
        if (data.success && data.report) {
            const report = data.report;
            let html = '<div class="alert alert-success">Debug analysis completed</div>';
            html += '<div class="card mt-3"><div class="card-body">';
            html += `<h6>Log Analysis Summary</h6>`;
            html += `<p>Errors: ${report.log_analysis?.total_errors || 0}, Warnings: ${report.log_analysis?.total_warnings || 0}</p>`;
            
            if (report.ai_debugging_assistance) {
                html += `<h6 class="mt-3">AI Debugging Assistance</h6>`;
                if (typeof report.ai_debugging_assistance === 'string') {
                    html += `<pre class="bg-light p-3">${report.ai_debugging_assistance}</pre>`;
                } else {
                    html += `<pre class="bg-light p-3">${JSON.stringify(report.ai_debugging_assistance, null, 2)}</pre>`;
                }
            }
            
            html += '</div></div>';
            resultsEl.innerHTML = html;
        } else {
            resultsEl.innerHTML = `<div class="alert alert-danger">Error: ${data.error || 'Debug analysis failed'}</div>`;
        }
    } catch (error) {
        console.error('Error running debug analysis:', error);
        resultsEl.innerHTML = '<div class="alert alert-danger">Failed to run debug analysis</div>';
        general_error('Failed to run debug analysis');
    }
}

async function analyzeLogs() {
    const resultsEl = document.getElementById('c2000-logs-results');
    resultsEl.innerHTML = '<div class="alert alert-info">Analyzing logs...</div>';
    
    try {
        const logFiles = document.getElementById('c2000-logs-files').value.split(',').map(f => f.trim()).filter(f => f);
        
        if (logFiles.length === 0) {
            resultsEl.innerHTML = '<div class="alert alert-warning">Please provide log file paths</div>';
            return;
        }
        
        const response = await fetch('/tab-c2000-logs-analyze', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ log_files: logFiles })
        });
        
        const data = await response.json();
        
        if (data.success && data.analysis) {
            const analysis = data.analysis;
            let html = '<div class="card"><div class="card-body">';
            html += `<h6>Analysis Results</h6>`;
            html += `<p>Errors: ${analysis.errors.length}, Warnings: ${analysis.warnings.length}</p>`;
            
            if (analysis.errors.length > 0) {
                html += '<h6 class="mt-3 text-danger">Errors</h6><ul>';
                analysis.errors.slice(0, 10).forEach(err => {
                    html += `<li><code>${err.file}:${err.line}</code> - ${err.content}</li>`;
                });
                html += '</ul>';
            }
            
            if (analysis.warnings.length > 0) {
                html += '<h6 class="mt-3 text-warning">Warnings</h6><ul>';
                analysis.warnings.slice(0, 10).forEach(warn => {
                    html += `<li><code>${warn.file}:${warn.line}</code> - ${warn.content}</li>`;
                });
                html += '</ul>';
            }
            
            html += '</div></div>';
            resultsEl.innerHTML = html;
        } else {
            resultsEl.innerHTML = `<div class="alert alert-danger">Error: ${data.error || 'Log analysis failed'}</div>`;
        }
    } catch (error) {
        console.error('Error analyzing logs:', error);
        resultsEl.innerHTML = '<div class="alert alert-danger">Failed to analyze logs</div>';
        general_error('Failed to analyze logs');
    }
}

// Global functions for onclick handlers
window.buildProject = async function(projectName) {
    try {
        const response = await fetch('/tab-c2000-build', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ project_name: projectName, configuration: 'FLASH' })
        });
        const data = await response.json();
        if (data.success) {
            alert(`Build initiated for ${projectName}`);
        } else {
            alert(`Error: ${data.error || 'Build failed'}`);
        }
    } catch (error) {
        general_error('Failed to build project');
    }
};

window.flashProject = async function(projectName) {
    try {
        const response = await fetch('/tab-c2000-flash', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ project_name: projectName })
        });
        const data = await response.json();
        if (data.success) {
            alert(`Flash initiated for ${projectName}`);
        } else {
            alert(`Error: ${data.error || 'Flash failed'}`);
        }
    } catch (error) {
        general_error('Failed to flash project');
    }
};

window.createProjectFromExample = async function(exampleName, examplePath) {
    try {
        const response = await fetch('/tab-c2000-project-create', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ project_name: exampleName, example_path: examplePath })
        });
        const data = await response.json();
        if (data.success) {
            alert(`Project creation initiated for ${exampleName}`);
            await loadProjects();
        } else {
            alert(`Error: ${data.error || 'Project creation failed'}`);
        }
    } catch (error) {
        general_error('Failed to create project');
    }
};

export function tab_switched_here() {
    // Refresh data when tab is switched to
    loadConfig();
    loadProjects();
    loadExamples();
}

export function tab_switched_away() {
    // Cleanup if needed
}

export function tab_update_each_couple_of_seconds() {
    // Periodic updates if needed
}










