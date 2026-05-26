'use strict';

const { app, BrowserWindow, ipcMain, dialog, shell, Tray, Menu, nativeImage, protocol, net } = require('electron');

// Must be called before app.ready — registers 'app://' as a secure standard scheme
// so absolute paths like /new_logo.png resolve correctly against it.
protocol.registerSchemesAsPrivileged([{
  scheme: 'app',
  privileges: { secure: true, standard: true, supportFetchAPI: true, corsEnabled: true },
}]);
const path = require('path');
const fs = require('fs');
const { spawn, spawnSync, execSync } = require('child_process');
const http = require('http');
const os = require('os');

// ─── Platform helpers ─────────────────────────────────────────────────────────
const IS_WIN = process.platform === 'win32';

// ─── Paths ────────────────────────────────────────────────────────────────────
const IS_DEV = process.argv.includes('--dev') || !app.isPackaged;
const REPO_ROOT = IS_DEV ? path.resolve(__dirname, '..') : null;

const CONFIG_DIR  = IS_WIN
  ? path.join(process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming'), 'craftifai')
  : path.join(os.homedir(), '.config', 'craftifai');
const CONFIG_FILE = path.join(CONFIG_DIR, 'settings.json');
const LOG_DIR     = IS_WIN
  ? path.join(CONFIG_DIR, 'logs')
  : path.join(os.homedir(), '.local', 'share', 'craftifai', 'logs');

/** Resolve a bundled resource path (dev vs packaged AppImage). */
function res(...parts) {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, ...parts);
  }
  // Dev mode: resources sit next to craftifai-desktop/ inside the repo.
  // Map logical resource names to their actual dev-mode locations.
  const DEV_PATH_MAP = {
    'api': ['api-bundle'],
    'gui': ['refact-agent', 'gui', 'dist', 'app'],
  };
  const mapped = DEV_PATH_MAP[parts[0]]
    ? [...DEV_PATH_MAP[parts[0]], ...parts.slice(1)]
    : parts;
  return path.join(REPO_ROOT, ...mapped);
}

// ─── Logging ─────────────────────────────────────────────────────────────────
fs.mkdirSync(LOG_DIR, { recursive: true });
const logStream = fs.createWriteStream(path.join(LOG_DIR, 'craftifai.log'), { flags: 'a' });

function log(tag, msg) {
  const line = `[${new Date().toISOString()}] [${tag}] ${msg}`;
  console.log(line);
  logStream.write(line + '\n');
}

// ─── Settings ─────────────────────────────────────────────────────────────────
function loadSettings() {
  try {
    if (fs.existsSync(CONFIG_FILE)) {
      return JSON.parse(fs.readFileSync(CONFIG_FILE, 'utf8'));
    }
  } catch (e) {
    log('SETTINGS', `Failed to read settings: ${e.message}`);
  }
  return {};
}

function saveSettings(settings) {
  fs.mkdirSync(CONFIG_DIR, { recursive: true });
  fs.writeFileSync(CONFIG_FILE, JSON.stringify(settings, null, 2), 'utf8');
  log('SETTINGS', `Saved to ${CONFIG_FILE}`);
}

// ─── Process Management ───────────────────────────────────────────────────────
let apiProcess  = null;
let lspProcess  = null;
let mainWindow  = null;
let tray        = null;
let isQuitting  = false;

function requestFullQuit() {
  if (isQuitting) return;
  isQuitting = true;
  app.quit();
}

// ─── Python venv management ───────────────────────────────────────────────────
const VENV_DIR = path.join(CONFIG_DIR, 'venv');

function getPythonBin() {
  return IS_WIN
    ? path.join(VENV_DIR, 'Scripts', 'python.exe')
    : path.join(VENV_DIR, 'bin', 'python3');
}

function pythonWorks(executable, prefixArgs = []) {
  try {
    const result = spawnSync(executable, [...prefixArgs, '--version'], {
      encoding: 'utf8',
      timeout: 10000,
      windowsHide: true,
    });
    if (result.status !== 0) return false;
    const out = `${result.stdout || ''}${result.stderr || ''}`;
    return /Python 3\.\d+/i.test(out);
  } catch (_) {
    return false;
  }
}

function collectWindowsPythonCandidates() {
  const seen = new Set();
  const candidates = [];

  const add = (executable, prefixArgs = []) => {
    const key = `${executable}\0${prefixArgs.join('\0')}`;
    if (seen.has(key)) return;
    seen.add(key);
    candidates.push({ executable, prefixArgs });
  };

  // Python Launcher registrations (py -0p), skipping broken installs.
  try {
    const listing = execSync('py -0p', { encoding: 'utf8', timeout: 10000 });
    for (const line of listing.split(/\r?\n/)) {
      const match = line.match(/([A-Za-z]:\\[^\s]+\\python\.exe)\s*$/i);
      if (match) add(match[1]);
    }
  } catch (_) {}

  // Standard per-user and system install locations.
  const localAppData = process.env.LOCALAPPDATA || '';
  const programFiles = process.env.ProgramFiles || 'C:\\Program Files';
  for (const ver of ['314', '313', '312', '311', '310']) {
    add(path.join(localAppData, 'Programs', 'Python', `Python${ver}`, 'python.exe'));
    add(path.join(programFiles, `Python${ver}`, 'python.exe'));
  }

  // Resolve PATH shims to real executables where possible.
  for (const cmd of ['python3.14', 'python3.13', 'python3.12', 'python3.11', 'python3.10', 'python3', 'python']) {
    try {
      const resolved = execSync(`where.exe ${cmd}`, { encoding: 'utf8', timeout: 5000 })
        .split(/\r?\n/)
        .map(s => s.trim())
        .filter(Boolean);
      for (const exe of resolved) {
        if (/\\WindowsApps\\/i.test(exe)) continue; // skip Store stub
        add(exe);
      }
    } catch (_) {}
  }

  // py.exe launcher as a last-resort fallback.
  const pyExe = path.join(process.env.WINDIR || 'C:\\Windows', 'py.exe');
  if (fs.existsSync(pyExe)) add(pyExe, ['-3']);

  return candidates;
}

/**
 * Returns { executable, prefixArgs } for creating the managed venv.
 * Throws if no working Python 3 is found.
 */
function getSystemPython() {
  if (!IS_WIN) {
    for (const cmd of ['python3', 'python']) {
      if (pythonWorks(cmd)) return { executable: cmd, prefixArgs: [] };
    }
    throw new Error('Python 3 not found. Install Python 3.10+ and ensure it is on PATH.');
  }

  const tried = [];
  for (const candidate of collectWindowsPythonCandidates()) {
    tried.push(candidate.prefixArgs.length
      ? `${candidate.executable} ${candidate.prefixArgs.join(' ')}`
      : candidate.executable);
    if (pythonWorks(candidate.executable, candidate.prefixArgs)) {
      return candidate;
    }
  }

  throw new Error(
    'Python 3 not found. Install Python 3.10+ from https://www.python.org/downloads/ ' +
    '(check "Add python.exe to PATH"), then click Retry.\n\n' +
    `Tried: ${tried.slice(0, 8).join('; ')}${tried.length > 8 ? '…' : ''}`
  );
}

/** Directory containing a working python.exe — used to extend PATH for ESP-IDF export.ps1. */
function getSystemPythonBinDir() {
  try {
    const py = getSystemPython();
    if (py.prefixArgs.length === 0 && py.executable.toLowerCase().endsWith('python.exe')) {
      return path.dirname(py.executable);
    }
    if (path.basename(py.executable).toLowerCase() === 'py.exe') {
      const result = spawnSync(py.executable, [...py.prefixArgs, '-c', 'import sys; print(sys.executable)'], {
        encoding: 'utf8',
        timeout: 10000,
        windowsHide: true,
      });
      if (result.status === 0 && result.stdout.trim()) {
        return path.dirname(result.stdout.trim());
      }
    }
  } catch (_) {}
  return null;
}

/**
 * Ensure the managed venv exists and all requirements are installed.
 * Calls statusCb(msg) with progress messages for the loading screen.
 */
async function ensurePythonVenv(statusCb) {
  const apiDir  = res('api');
  const reqFile = path.join(apiDir, 'requirements.txt');
  const stampFile = path.join(VENV_DIR, '.installed-stamp');

  // Check if requirements.txt changed since last install
  let reqHash = '';
  try { reqHash = require('crypto').createHash('md5').update(fs.readFileSync(reqFile)).digest('hex'); } catch (_) {}

  let stampHash = '';
  try { stampHash = fs.readFileSync(stampFile, 'utf8').trim(); } catch (_) {}

  if (stampHash === reqHash && fs.existsSync(getPythonBin())) {
    log('VENV', 'Venv up to date, skipping install');
    return;
  }

  // Create venv if it doesn't exist
  if (!fs.existsSync(getPythonBin())) {
    statusCb('Creating Python environment (first run, one-time setup)…');
    log('VENV', `Creating venv at ${VENV_DIR}`);
    await new Promise((resolve, reject) => {
      let sysPy;
      try {
        sysPy = getSystemPython();
      } catch (err) {
        reject(err);
        return;
      }
      log('VENV', `Using system Python: ${sysPy.prefixArgs.length ? sysPy.prefixArgs.join(' ') + ' ' : ''}${sysPy.executable}`);
      const p = spawn(sysPy.executable, [...sysPy.prefixArgs, '-m', 'venv', VENV_DIR]);
      p.stderr.on('data', d => log('VENV', d.toString().trimEnd()));
      p.on('exit', code => code === 0 ? resolve() : reject(new Error(`venv creation failed (exit ${code}). Make sure Python 3 is installed from python.org`)));
    });
    log('VENV', 'Venv created');
  }

  // Install requirements
  statusCb('Installing Python dependencies (first run, ~1–2 min)…');
  log('VENV', `Installing requirements from ${reqFile}`);
  await new Promise((resolve, reject) => {
    const pip = IS_WIN
      ? path.join(VENV_DIR, 'Scripts', 'pip.exe')
      : path.join(VENV_DIR, 'bin', 'pip');
    const p = spawn(pip, [
      'install', '--quiet', '--disable-pip-version-check',
      '-r', reqFile,
    ]);
    const pipLog = fs.createWriteStream(path.join(LOG_DIR, 'pip-install.log'), { flags: 'a' });
    p.stdout.on('data', d => pipLog.write(d));
    p.stderr.on('data', d => { pipLog.write(d); log('PIP', d.toString().trimEnd()); });
    p.on('exit', code => {
      if (code === 0) {
        // Write stamp so we skip this next time
        try { fs.writeFileSync(stampFile, reqHash); } catch (_) {}
        resolve();
      } else {
        reject(new Error(`pip install failed (exit ${code}). See ${path.join(LOG_DIR, 'pip-install.log')}`));
      }
    });
  });
  log('VENV', 'Requirements installed');
}

function spawnAPI(settings) {
  const apiDir = res('api');
  const python = getPythonBin();
  const env = {
    ...process.env,
    OPENAI_API_KEY:   settings.openai_api_key,
    REFACT_CACHE_DIR: CONFIG_DIR,
  };

  log('API', `Starting via venv Python: ${python}`);
  const proc = spawn(python, [
    '-m', 'uvicorn', 'refactapi:app',
    '--host', '0.0.0.0',
    '--port', '8002',
    '--log-level', 'warning',
  ], { cwd: apiDir, env });

  const apiLog = fs.createWriteStream(path.join(LOG_DIR, 'api.log'), { flags: 'a' });
  proc.stdout.on('data', d => { apiLog.write(d); log('API', d.toString().trimEnd()); });
  proc.stderr.on('data', d => { apiLog.write(d); log('API', d.toString().trimEnd()); });
  proc.on('exit', code => log('API', `Process exited with code ${code}`));
  return proc;
}

/**
 * Write ~/.config/craftifai/esp32_tools.yaml from current settings.
 * The Python API (REFACT_CACHE_DIR=CONFIG_DIR) reads this file for /v1/esp32-config.
 * refact-lsp calls that endpoint to get esp_idf_path, projects_path, serial port, etc.
 * Without this file the API returns 404 and ALL esp32_project/build/flash tools fail.
 */
function syncEsp32Config(settings) {
  const idfExport  = settings.idf_export_sh || '';
  const idfPath    = idfExport ? path.dirname(idfExport) : '';
  const projectsPath = settings.project_dir
    || path.join(os.homedir(), 'craftifai-workspace');

  // YAML double-quoted scalars treat backslashes as escapes (e.g. "\E" is invalid),
  // so always single-quote paths and escape single quotes.
  const yq = (v) => `'${String(v ?? '').replace(/'/g, "''")}'`;

  // Write to ALL locations refact-lsp checks:
  //   1. ~/.config/craftifai/  (our REFACT_CACHE_DIR, read by the Python API)
  //   2. ~/.cache/refact/      (hardcoded file fallback in config.rs)
  //   3. ~/.config/refact/     (refact-lsp migrates from ~/.cache/refact/ to here on first run,
  //                             then reads ONLY this location going forward — confirmed via log:
  //                             "cannot migrate ... destination exists")
  const configPaths = IS_WIN
    ? [
        path.join(CONFIG_DIR, 'esp32_tools.yaml'),
        path.join(process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local'), 'refact', 'esp32_tools.yaml'),
        path.join(process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming'), 'refact', 'esp32_tools.yaml'),
        path.join(os.homedir(), '.cache',  'refact', 'esp32_tools.yaml'),
        path.join(os.homedir(), '.config', 'refact', 'esp32_tools.yaml'),
      ]
    : [
        path.join(CONFIG_DIR, 'esp32_tools.yaml'),
        path.join(os.homedir(), '.cache',  'refact', 'esp32_tools.yaml'),
        path.join(os.homedir(), '.config', 'refact', 'esp32_tools.yaml'),
      ];

  const defaultPort = IS_WIN ? 'COM3' : '/dev/ttyUSB0';

  const yaml = `# Auto-generated by CraftifAI desktop app — do not edit by hand
# To update, change settings inside the app (Preferences)
esp32_config:
  esp_idf_path: ${yq(idfPath)}
  projects_path: ${yq(projectsPath)}
  default_target: "esp32s3"
  default_serial_port: "${defaultPort}"
  default_baud_rate: 115200
  default_flash_baud_rate: 115200
  default_monitor_baud_rate: 115200
  default_flash_mode: "dio"
  default_flash_freq: "80m"
  default_flash_size: "16MB"
  ota_enabled: false
  ota_partition_scheme: "default"
  cloud_provider: "none"
  mqtt_broker: ""

tools:
  esp32_project:
    enabled: true
  esp32_build:
    enabled: true
  esp32_device:
    enabled: true
  esp32_config:
    enabled: true
  esp32_component:
    enabled: true
  esp32_analyze:
    enabled: true
`;

  for (const configPath of configPaths) {
    fs.mkdirSync(path.dirname(configPath), { recursive: true });
    fs.writeFileSync(configPath, yaml, 'utf8');
    log('CONFIG', `esp32_tools.yaml → ${configPath}`);
  }
  log('CONFIG', `  esp_idf_path=${idfPath || '(not set)'},  projects_path=${projectsPath}`);
}

/**
 * Write a project-level indexing.yaml into CONFIG_DIR the first time it is
 * missing.  The file blocks directories that are never useful to index in an
 * ESP32 / firmware workspace (build artefacts, toolchain caches, JS deps, …).
 * Passing it via --indexing-yaml means refact-lsp never tries to walk or AST-
 * parse those trees, which dramatically reduces first-scan time on Windows
 * when the AI Agent Sandbox points at a large parent folder.
 */
function ensureIndexingYaml() {
  const indexingYamlPath = path.join(CONFIG_DIR, 'indexing.yaml');
  if (fs.existsSync(indexingYamlPath)) return indexingYamlPath;

  const content = [
    '# Auto-generated by CraftifAI — edit to customise what the AI indexes.',
    '# Paths are matched as glob patterns relative to each workspace root.',
    'blocklist:',
    '  # Hidden directories (.git, .svn, .idea, …)',
    '  - "*/.*"',
    '  # Generic build / output dirs',
    '  - "*/target/*"',
    '  - "*/build/*"',
    '  - "*/dist/*"',
    '  - "*/out/*"',
    '  - "*/obj/*"',
    '  - "*/bin/*"',
    '  - "*/pkg/*"',
    '  - "*/lib/*"',
    '  # Dependency caches',
    '  - "*/node_modules/*"',
    '  - "*/vendor/*"',
    '  - "*/venv/*"',
    '  - "*/env/*"',
    '  - "*/.venv/*"',
    '  # Temp / log dirs',
    '  - "*/tmp/*"',
    '  - "*/temp/*"',
    '  - "*/logs/*"',
    '  - "*/coverage/*"',
    '  - "*/backup/*"',
    '  # Python bytecode',
    '  - "*/__pycache__/*"',
    '  # Build-system internals',
    '  - "*/.gradle/*"',
    '  - "*/_trajectories/*"',
    '  # ESP-IDF / ESP32 toolchain artefacts',
    '  - "*/.espressif/*"',
    '  - "*/managed_components/*"',
    '  - "*/bootloader_build/*"',
    '  - "*/x_tools/*"',
    '  - "*/.idf_tools/*"',
    '  - "*/esp-idf/*"',
    '  - "*/xtensa-esp*/*"',
  ].join('\n') + '\n';

  try {
    fs.mkdirSync(CONFIG_DIR, { recursive: true });
    fs.writeFileSync(indexingYamlPath, content, 'utf8');
    log('MAIN', `Created indexing.yaml at ${indexingYamlPath}`);
  } catch (e) {
    log('MAIN', `WARNING: could not write indexing.yaml: ${e.message}`);
    return null;
  }
  return indexingYamlPath;
}

function spawnLSP(settings) {
  const lspBinName = IS_WIN ? 'refact-lsp.exe' : 'refact-lsp';
  const lspBin = res('bin', lspBinName);

  if (!fs.existsSync(lspBin)) {
    const hint = IS_WIN ? 'scripts\\build-app.ps1' : 'scripts/build-app.sh';
    throw new Error(`refact-lsp binary not found at: ${lspBin}\nRun ${hint} first.`);
  }

  const workspaceFolder = settings.project_dir
    || path.join(os.homedir(), 'craftifai-workspace');
  fs.mkdirSync(workspaceFolder, { recursive: true });

  const board = settings.board_definition || 'esp32-s3-DevKitM-1-N16R8';

  // Custom indexing blocklist — created once by ensureIndexingYaml().
  // Keeps the file-walk scope small even without AST indexing.
  const indexingYamlPath = ensureIndexingYaml();

  // --ast is intentionally omitted: enabling it triggers a full file-content
  // parse that can take 10-15 minutes on a large sandbox on Windows (NTFS I/O
  // + ReadDirectoryChangesW watcher).  The agent works fine without it for
  // firmware generation (file read/write, idf.py build, flash, ESP32 tools).
  // If you want symbol search re-add --ast here; pair it with --ast-permanent
  // so subsequent launches load from the cache instead of re-parsing.
  const lspArgs = [
    '--address-url',      'http://127.0.0.1:8002',
    '--api-key',          settings.openai_api_key || '',
    '--logs-stderr',
    '--http-port=8486',
    '--platform',         'esp32',
    '--board-definition', board,
    '--workspace-folder', workspaceFolder,
    ...(indexingYamlPath ? ['--indexing-yaml', indexingYamlPath] : []),
  ];

  let proc;

  if (IS_WIN) {
    // On Windows, generate a PowerShell wrapper that activates ESP-IDF via export.ps1
    // then launches refact-lsp in the same environment.
    const wrapperPath = path.join(CONFIG_DIR, 'run-lsp.ps1');
    const escapedArgs = lspArgs.map(a => `'${a.replace(/'/g, "''")}'`).join(' `\n  ');

    let script = '# Auto-generated by CraftifAI — do not edit\n';
    script += '$ErrorActionPreference = "Continue"\n\n';
    const pythonBinDir = getSystemPythonBinDir();
    if (pythonBinDir) {
      script += `# Ensure Python is on PATH for ESP-IDF export.ps1\n`;
      script += `$env:Path = "${pythonBinDir};${pythonBinDir}\\Scripts;" + $env:Path\n\n`;
    }
    if (settings.idf_export_sh && fs.existsSync(settings.idf_export_sh)) {
      script += `# Activate ESP-IDF environment (best-effort — agent still starts if this fails)\n`;
      script += `try {\n  & "${settings.idf_export_sh}"\n} catch {\n  Write-Warning "ESP-IDF export failed (agent will still start): $_"\n}\n\n`;
      log('LSP', `Wrapper will run IDF export: ${settings.idf_export_sh}`);
    } else {
      log('LSP', 'WARNING: idf_export_sh not set or not found — idf.py commands will fail');
    }
    script += `$env:REFACT_ESP32_CONFIG_URL = "http://127.0.0.1:8002/v1/esp32-config"\n\n`;
    script += `# Launch agent\n`;
    script += `& "${lspBin}" ${escapedArgs}\n`;

    fs.writeFileSync(wrapperPath, script, 'utf8');
    log('LSP', `Wrapper written to: ${wrapperPath}`);
    log('LSP', `board=${board}  project_dir=${workspaceFolder}`);

    proc = spawn('powershell.exe', [
      '-ExecutionPolicy', 'Bypass', '-NoProfile', '-File', wrapperPath,
    ], {
      env: { ...process.env, OPENAI_API_KEY: settings.openai_api_key },
    });
  } else {
    // On Linux/macOS, generate a bash wrapper that sources export.sh
    const wrapperPath = path.join(CONFIG_DIR, 'run-lsp.sh');
    const quotedArgs  = lspArgs.map(a => `'${a.replace(/'/g, "'\\''")}'`).join(' \\\n  ');

    let script = '#!/usr/bin/env bash\n\n';
    if (settings.idf_export_sh && fs.existsSync(settings.idf_export_sh)) {
      script += `. "${settings.idf_export_sh}"\n\n`;
      log('LSP', `Wrapper will source IDF: ${settings.idf_export_sh}`);
    } else {
      log('LSP', 'WARNING: idf_export_sh not set or not found — idf.py commands will fail');
    }
    script += `export REFACT_ESP32_CONFIG_URL=http://127.0.0.1:8002/v1/esp32-config\n\n`;
    script += `exec "${lspBin}" \\\n  ${quotedArgs}\n`;

    fs.writeFileSync(wrapperPath, script, { mode: 0o755 });
    log('LSP', `Wrapper written to: ${wrapperPath}`);
    log('LSP', `board=${board}  project_dir=${workspaceFolder}`);

    proc = spawn('bash', [wrapperPath], {
      env: { ...process.env, OPENAI_API_KEY: settings.openai_api_key },
    });
  }

  const lspLog = fs.createWriteStream(path.join(LOG_DIR, 'lsp.log'), { flags: 'a' });
  proc.stdout.on('data', d => { lspLog.write(d); });
  proc.stderr.on('data', d => { lspLog.write(d); log('LSP', d.toString().trimEnd()); });
  proc.on('exit', code => log('LSP', `Process exited with code ${code}`));
  return proc;
}

// ─── API Health Check ─────────────────────────────────────────────────────────
function waitForAPI(timeoutMs = 60000) {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + timeoutMs;
    const check = () => {
      const req = http.get('http://127.0.0.1:8002/refact-caps', res => {
        if (res.statusCode === 200) { resolve(); return; }
        res.resume();
        scheduleNext();
      });
      req.on('error', scheduleNext);
      req.end();
    };
    const scheduleNext = () => {
      if (Date.now() >= deadline) { reject(new Error('API did not start within timeout')); return; }
      setTimeout(check, 1200);
    };
    check();
  });
}

function waitForLSP(timeoutMs = 90000) {
  return new Promise((resolve, reject) => {
    const deadline = Date.now() + timeoutMs;
    const check = () => {
      const req = http.get('http://127.0.0.1:8486/v1/ping', res => {
        if (res.statusCode === 200) { resolve(); return; }
        res.resume();
        scheduleNext();
      });
      req.on('error', scheduleNext);
      req.end();
    };
    const scheduleNext = () => {
      if (Date.now() >= deadline) {
        reject(new Error('AI agent did not restart within timeout. Check logs for LSP errors.'));
        return;
      }
      setTimeout(check, 1500);
    };
    check();
  });
}

// ─── Window Helpers ───────────────────────────────────────────────────────────
function createWindow(opts) {
  return new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 960,
    minHeight: 600,
    backgroundColor: '#0f1117',
    titleBarStyle: 'default',
    show: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      webSecurity: false, // needed: file:// renderer → http://localhost:8486 API calls
    },
    ...opts,
  });
}

function sendToRenderer(channel, ...args) {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send(channel, ...args);
  }
}

/**
 * POST /v1/lsp-initialize with the previously saved sandbox path so refact-lsp
 * resumes watching that folder on startup.  With --ast-permanent the engine
 * loads the cached index and only re-parses files that changed, keeping the
 * restart latency small even for large sandboxes.
 */
function restoreSandboxPath(folderPath) {
  let folderUrl;
  if (/^[A-Za-z]:[\\/]/.test(folderPath)) {
    folderUrl = `file:///${folderPath.replace(/\\/g, '/')}`;
  } else {
    folderUrl = folderPath.startsWith('/') ? `file://${folderPath}` : `file:///${folderPath}`;
  }

  const body = JSON.stringify({ project_roots: [folderUrl] });
  const req = http.request({
    hostname: '127.0.0.1',
    port: 8486,
    path: '/v1/lsp-initialize',
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
  }, (res) => {
    res.resume();
    log('MAIN', `Restored sandbox path on startup: ${folderPath} (HTTP ${res.statusCode})`);
  });
  req.on('error', (e) => log('MAIN', `Could not restore sandbox path: ${e.message}`));
  req.write(body);
  req.end();
}

// ─── Launch App ───────────────────────────────────────────────────────────────
async function launchApp(incomingSettings) {
  // On first run, write a placeholder settings.json so downstream code has
  // defined values. The user will fill in the real values from the Welcome page
  // after login — those are saved via the 'save-wizard-settings' IPC handler.
  let settings = incomingSettings;
  if (!fs.existsSync(CONFIG_FILE)) {
    const placeholder = {
      openai_api_key:   '',
      idf_export_sh:    null,
      board_definition: null,
      project_dir:      null,
      setup_complete:   false,
    };
    saveSettings(placeholder);
    settings = placeholder;
    log('MAIN', 'First run — placeholder settings.json written');
  }

  // Show loading page immediately
  mainWindow.loadFile(path.join(__dirname, 'renderer', 'loading.html'));
  mainWindow.once('ready-to-show', () => mainWindow.show());

  try {
    // 1. Sync esp32_tools.yaml so /v1/esp32-config returns correct paths
    syncEsp32Config(settings);

    // 2. Ensure Python venv + dependencies are installed
    await ensurePythonVenv(msg => sendToRenderer('status', msg));

    // 3. Start Python API
    sendToRenderer('status', 'Starting API server…');
    apiProcess = spawnAPI(settings);

    sendToRenderer('status', 'Waiting for API to be ready…');
    await waitForAPI();
    log('MAIN', 'API is up');

    // Inject the selected board definition into the LLM context
    notifyActiveBoardToAPI(settings.board_definition || 'esp32-s3-DevKitM-1-N16R8');

    // 4. Start refact-lsp — wrapper script sources IDF in same shell (like run-all-docker-esp32.sh)
    sendToRenderer('status', 'Starting AI agent…');
    lspProcess = spawnLSP(settings);

    // Give LSP 2s to bind its HTTP port
    await new Promise(r => setTimeout(r, 2000));
    log('MAIN', 'All services started');

    // Restore previously saved sandbox folder so the agent resumes indexing
    // without the user having to re-open the wizard on every launch.
    // We fire-and-forget: if it fails the wizard still lets the user set it again.
    if (settings.sandbox_path) {
      restoreSandboxPath(settings.sandbox_path);
    }

    // 4. Load the main GUI via custom protocol (fixes absolute asset paths like /new_logo.png)
    log('MAIN', `Loading GUI via app://localhost/ → ${res('gui')}`);
    mainWindow.loadURL('app://localhost/index.html');
    mainWindow.setTitle('CraftifAI ESP32 Agent');

  } catch (err) {
    log('MAIN', `Launch failed: ${err.message}`);
    mainWindow.loadFile(path.join(__dirname, 'renderer', 'error.html'));
    mainWindow.webContents.once('did-finish-load', () => {
      sendToRenderer('error-message', err.message);
    });
  }
}

// ─── System Tray ─────────────────────────────────────────────────────────────
function setupTray() {
  const iconPath = path.join(__dirname, 'assets', 'tray-icon.png');
  if (!fs.existsSync(iconPath)) return;

  tray = new Tray(nativeImage.createFromPath(iconPath).resize({ width: 22, height: 22 }));
  const menu = Menu.buildFromTemplate([
    { label: 'Show App', click: () => { if (mainWindow) mainWindow.show(); } },
    { type: 'separator' },
    { label: 'Open Log Folder', click: () => shell.openPath(LOG_DIR) },
    { type: 'separator' },
    { label: 'Quit', click: () => requestFullQuit() },
  ]);
  tray.setToolTip('CraftifAI ESP32 Agent');
  tray.setContextMenu(menu);
  tray.on('activate', () => { if (mainWindow) mainWindow.show(); });
}

function setupAppMenu() {
  const menu = Menu.buildFromTemplate([
    {
      label: 'File',
      submenu: [
        { label: 'Exit', click: () => requestFullQuit() },
      ],
    },
  ]);
  Menu.setApplicationMenu(menu);
}

// ─── Board context injection ──────────────────────────────────────────────────
/**
 * Notify refactapi of the active board so it can inject board knowledge into
 * every LLM chat request from session start.
 */
function notifyActiveBoardToAPI(boardId) {
  if (!boardId) return;
  const body = JSON.stringify({ board_id: boardId });
  const req = http.request(
    { hostname: '127.0.0.1', port: 8002, path: '/v1/set-active-board', method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) } },
    (res) => {
      res.resume();
      log('MAIN', `set-active-board → ${boardId} (HTTP ${res.statusCode})`);
    }
  );
  req.on('error', (e) => log('MAIN', `set-active-board request failed: ${e.message}`));
  req.write(body);
  req.end();
}

// ─── IPC Handlers ─────────────────────────────────────────────────────────────
ipcMain.handle('get-settings', () => loadSettings());

// Called by the Welcome page after login to persist ESP32 configuration.
// Updates settings.json + esp32_tools.yaml immediately and restarts refact-lsp
// so CLI-only settings (board/workspace) apply without restarting the app.
ipcMain.handle('save-wizard-settings', async (_event, data) => {
  try {
    const current = loadSettings();
    const merged = { ...current, ...data, setup_complete: true };
    saveSettings(merged);
    // Ensure project_dir exists on disk
    if (merged.project_dir) {
      fs.mkdirSync(merged.project_dir, { recursive: true });
    }
    syncEsp32Config(merged);

    // Restart only LSP so --board-definition / --workspace-folder take effect now.
    try {
      if (lspProcess && lspProcess.pid) {
        log('MAIN', `Restarting LSP (pid ${lspProcess.pid}) to apply new settings…`);
        if (IS_WIN) {
          try { execSync(`taskkill /PID ${lspProcess.pid} /T /F`, { stdio: 'ignore' }); } catch (_) {}
        } else {
          try { lspProcess.kill('SIGTERM'); } catch (_) {}
        }
      }
      lspProcess = spawnLSP(merged);
      try {
        await waitForLSP();
        log('MAIN', 'Wizard settings saved; esp32_tools.yaml updated; LSP restarted and ready');
      } catch (e) {
        log('MAIN', `WARNING: LSP restarted but not reachable yet: ${e.message}`);
        return { ok: false, error: e.message };
      }
    } catch (e) {
      log('MAIN', `WARNING: LSP restart failed after saving wizard settings: ${e.message}`);
      log('MAIN', 'Wizard settings saved and esp32_tools.yaml updated (LSP not restarted)');
    }

    // Update the LLM board context to match the newly selected board
    notifyActiveBoardToAPI(merged.board_definition || 'esp32-s3-DevKitM-1-N16R8');

    return { ok: true };
  } catch (e) {
    log('MAIN', `save-wizard-settings failed: ${e.message}`);
    return { ok: false, error: e.message };
  }
});

ipcMain.handle('browse-folder', async () => {
  const result = await dialog.showOpenDialog(mainWindow, {
    title: 'Select Folder',
    properties: ['openDirectory', 'createDirectory'],
  });
  return result.canceled ? null : result.filePaths[0];
});

ipcMain.handle('browse-file', async (_event, opts = {}) => {
  const result = await dialog.showOpenDialog(mainWindow, {
    title: opts.title || 'Select File',
    properties: ['openFile'],
    filters: opts.filters || [{ name: 'All Files', extensions: ['*'] }],
  });
  return result.canceled ? null : result.filePaths[0];
});

ipcMain.handle('get-log-dir', () => LOG_DIR);

ipcMain.handle('open-logs', () => shell.openPath(LOG_DIR));

ipcMain.handle('reset-settings', () => {
  try { fs.unlinkSync(CONFIG_FILE); } catch (_) {}
  app.relaunch();
  app.exit(0);
});

// ─── App Lifecycle ─────────────────────────────────────────────────────────────
app.whenReady().then(async () => {
  // Serve the built GUI via app://localhost/ so absolute asset paths (e.g. /new_logo.png)
  // resolve correctly against the GUI directory instead of the filesystem root.
  const guiDir = res('gui');
  protocol.handle('app', (request) => {
    let urlPath = new URL(request.url).pathname;
    if (urlPath === '/' || urlPath === '') urlPath = '/index.html';
    const filePath = path.join(guiDir, urlPath);
    const fileUrl = IS_WIN ? `file:///${filePath.replace(/\\/g, '/')}` : `file://${filePath}`;
    return net.fetch(fileUrl);
  });

  mainWindow = createWindow();
  setupAppMenu();
  setupTray();

  const settings = loadSettings();
  log('MAIN', settings.setup_complete ? 'Settings found — launching app' : 'First run — launching with placeholder settings');
  await launchApp(settings);

  // Red close button should fully quit app and child services.
  mainWindow.on('close', e => {
    if (!isQuitting) {
      e.preventDefault();
      requestFullQuit();
    }
  });
});

app.on('before-quit', () => {
  isQuitting = true;
  log('MAIN', 'Shutting down services…');
  if (IS_WIN) {
    // On Windows, kill the entire process tree (wrapper + grandchildren like refact-lsp.exe)
    const { execSync } = require('child_process');
    for (const proc of [lspProcess, apiProcess]) {
      if (proc && proc.pid) {
        try { execSync(`taskkill /PID ${proc.pid} /T /F`, { stdio: 'ignore' }); } catch (_) {}
      }
    }
  } else {
    if (lspProcess) lspProcess.kill('SIGTERM');
    if (apiProcess) apiProcess.kill('SIGTERM');
  }
  lspProcess = null;
  apiProcess = null;
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) app.emit('ready');
  else if (mainWindow) mainWindow.show();
});
