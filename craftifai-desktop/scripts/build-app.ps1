# ─────────────────────────────────────────────────────────────────────────────
# CraftifAI ESP32 Agent — Full Desktop Build Script (Windows)
#
# Builds everything and produces:
#   craftifai-desktop\dist-packages\CraftifAI-ESP32-Agent-*.exe
#
# Prerequisites:
#   - Rust + cargo   (for refact-lsp)
#   - Node >= 18     (for GUI + Electron)
#   - Python >= 3.10 (for API bundle)
#
# Usage:
#   cd craftifai-desktop
#   powershell -ExecutionPolicy Bypass -File scripts\build-app.ps1
#
# Flags:
#   -SkipRust     Skip Rust build
#   -SkipGui      Skip GUI build
#   -SkipPython   Skip Python API bundle
#   -NsisOnly     Only build NSIS installer (skip portable)
# ─────────────────────────────────────────────────────────────────────────────

param(
    [switch]$SkipRust,
    [switch]$SkipGui,
    [switch]$SkipPython,
    [switch]$NsisOnly
)

$ErrorActionPreference = "Stop"

# ── Paths ─────────────────────────────────────────────────────────────────────
$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Path
$DesktopDir  = Split-Path -Parent $ScriptDir
$RepoRoot    = Split-Path -Parent $DesktopDir

$GuiDir      = Join-Path $RepoRoot "refact-agent\gui"
$EngineDir   = Join-Path $RepoRoot "refact-agent\engine"
$BinDir      = Join-Path $RepoRoot "bin"
$ApiBundleDir = Join-Path $RepoRoot "api-bundle"

function Write-Bold($msg) { Write-Host $msg -ForegroundColor White }
function Write-Info($msg) { Write-Host "  -> $msg" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Err($msg)  { Write-Host "  [ERR] $msg" -ForegroundColor Red }
function Write-Warn($msg) { Write-Host "  [WARN] $msg" -ForegroundColor Yellow }

function Stop-ListeningOnPort([int]$Port) {
    try {
        $pids = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique
        foreach ($procId in $pids) {
            if ($procId -and $procId -gt 0) {
                Write-Info "Stopping process on port $Port (PID $procId)..."
                Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
            }
        }
    } catch {
        # Get-NetTCPConnection may be unavailable; ignore.
    }
}

function Stop-CraftifDesktopProcesses {
    Stop-ListeningOnPort 8486
    Stop-ListeningOnPort 8002

    try {
        Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | ForEach-Object {
            $path = $_.ExecutablePath
            $name = $_.Name
            if (-not $path -and -not $name) { return }
            $shouldStop = $false
            if ($path -like "*\craftifai-desktop\dist-packages\*") { $shouldStop = $true }
            if ($name -eq "electron.exe" -and $path -like "*\craftifai-desktop\*") { $shouldStop = $true }
            if ($name -like "CraftifAI ESP32 Agent.exe") { $shouldStop = $true }
            if ($shouldStop) {
                Write-Info "Stopping $($name) (PID $($_.ProcessId))..."
                Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
            }
        }
    } catch {
        # WMI may fail on some systems; port-based stop above is the main safeguard.
    }
}

function Test-RepoOnOneDrive([string]$RootPath) {
    return ($RootPath -match "OneDrive")
}

function Get-ElectronFallbackOutputDir() {
    $fallbackOut = Join-Path $env:TEMP "craftifai-electron-dist"
    if (Test-Path $fallbackOut) {
        try {
            Remove-Item -Recurse -Force $fallbackOut -ErrorAction Stop
        } catch {
            $fallbackOut = Join-Path $env:TEMP ("craftifai-electron-dist-" + (Get-Date -Format "yyyyMMddHHmmss"))
        }
    }
    return $fallbackOut
}

function Copy-ElectronPackagesToDefault([string]$SourceDir, [string]$DefaultOut) {
    if ($SourceDir -eq $DefaultOut) { return }

    if (-not (Test-Path $DefaultOut)) {
        New-Item -ItemType Directory -Path $DefaultOut -Force | Out-Null
    }

    $copied = @()
    Get-ChildItem (Join-Path $SourceDir "*.exe") -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Copy-Item $_.FullName (Join-Path $DefaultOut $_.Name) -Force -ErrorAction Stop
            $copied += $_.Name
        } catch {
            Write-Warn "Could not copy $($_.Name) to dist-packages: $($_.Exception.Message)"
        }
    }

    if ($copied.Count -gt 0) {
        Write-Ok ("Copied installer(s) to dist-packages: " + ($copied -join ", "))
    }
}

function Resolve-ElectronOutputDir([string]$DesktopRoot, [string]$RepoRoot) {
    $defaultOut = Join-Path $DesktopRoot "dist-packages"
    $unpackDir = Join-Path $defaultOut "win-unpacked"

    # OneDrive Sync Service often locks app.asar under win-unpacked; build outside sync instead.
    if (Test-RepoOnOneDrive $RepoRoot) {
        $fallbackOut = Get-ElectronFallbackOutputDir
        Write-Warn "Repo is under OneDrive; electron-builder will use: $fallbackOut"
        Write-Host "  Installers are copied back to craftifai-desktop\dist-packages after packaging." -ForegroundColor Yellow
        Write-Host "  Tip: move the repo to C:\dev\ to avoid OneDrive locks during development." -ForegroundColor Yellow
        Write-Host ""
        return @{
            BuildDir = $fallbackOut
            DefaultOut = $defaultOut
        }
    }

    if (-not (Test-Path $unpackDir)) {
        return @{
            BuildDir = $defaultOut
            DefaultOut = $defaultOut
        }
    }

    Stop-CraftifDesktopProcesses
    Start-Sleep -Seconds 1

    for ($attempt = 1; $attempt -le 3; $attempt++) {
        try {
            Remove-Item -Recurse -Force $unpackDir -ErrorAction Stop
            Write-Ok "Cleared dist-packages\win-unpacked"
            return @{
                BuildDir = $defaultOut
                DefaultOut = $defaultOut
            }
        } catch {
            if ($attempt -lt 3) {
                Write-Warn "dist-packages\win-unpacked is locked (attempt $attempt/3); retrying..."
                Stop-CraftifDesktopProcesses
                Start-Sleep -Seconds 2
            }
        }
    }

    $fallbackOut = Get-ElectronFallbackOutputDir

    Write-Warn "Could not remove locked dist-packages\win-unpacked (OneDrive or a running app may hold app.asar)."
    Write-Warn "Building into fresh output folder: $fallbackOut"
    Write-Host ""
    Write-Host "  After build, you can delete the stale folder manually:" -ForegroundColor Yellow
    Write-Host "    craftifai-desktop\dist-packages\win-unpacked"
    Write-Host "  Tip: pause OneDrive sync or move the repo outside OneDrive\Desktop." -ForegroundColor Yellow
    Write-Host ""
    return @{
        BuildDir = $fallbackOut
        DefaultOut = $defaultOut
    }
}

Write-Bold "============================================"
Write-Bold " CraftifAI ESP32 Agent - Desktop Build (Win)"
Write-Bold "============================================"
Write-Host ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 1 — Build refact-lsp (Rust)
# ─────────────────────────────────────────────────────────────────────────────
if (-not $SkipRust) {
    Write-Bold "Phase 1/4 - Building refact-lsp (Rust)"
    Write-Info "Running: cargo build --release"
    Push-Location $EngineDir
    cargo build --release
    if ($LASTEXITCODE -ne 0) { Write-Err "cargo build failed"; exit 1 }
    Pop-Location

    if (-not (Test-Path $BinDir)) { New-Item -ItemType Directory -Path $BinDir -Force | Out-Null }
    Copy-Item (Join-Path $EngineDir "target\release\refact-lsp.exe") (Join-Path $BinDir "refact-lsp.exe") -Force
    $size = (Get-Item (Join-Path $BinDir "refact-lsp.exe")).Length / 1MB
    Write-Ok ("refact-lsp.exe -> $BinDir ({0:N1} MB)" -f $size)
} else {
    Write-Bold "Phase 1/4 - Skipping Rust build (-SkipRust)"
    $lspPath = Join-Path $BinDir "refact-lsp.exe"
    if (-not (Test-Path $lspPath)) {
        Write-Err "refact-lsp.exe not found at $lspPath"
        Write-Host "      Build it: cd refact-agent\engine && cargo build --release && copy target\release\refact-lsp.exe ..\..\bin\"
        exit 1
    }
    Write-Ok "Using existing: $lspPath"
}
Write-Host ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 2 — Build GUI (React -> static files)
# ─────────────────────────────────────────────────────────────────────────────
if (-not $SkipGui) {
    Write-Bold "Phase 2/4 - Building GUI (React -> static)"
    Push-Location $GuiDir

    if (-not (Test-Path "node_modules")) {
        Write-Info "Installing npm dependencies..."
        npm ci --prefer-offline
        if ($LASTEXITCODE -ne 0) { Write-Err "npm ci failed"; exit 1 }
    }

    Write-Info "Building standalone app bundle (vite.app.config.ts)..."
    $env:VITE_REFACT_LSP_URL = "http://127.0.0.1:8486"
    $env:VITE_UPLOAD_API_URL = "http://127.0.0.1:8002"
    $env:VITE_EMBEDDED_MODE = "true"
    npx vite build --config vite.app.config.ts
    if ($LASTEXITCODE -ne 0) { Write-Err "GUI build failed"; exit 1 }

    Pop-Location
    Write-Ok "GUI -> $GuiDir\dist\app\"
} else {
    Write-Bold "Phase 2/4 - Skipping GUI build (-SkipGui)"
    if (-not (Test-Path (Join-Path $GuiDir "dist\app"))) {
        Write-Err "GUI dist not found at $GuiDir\dist\app"
        exit 1
    }
    Write-Ok "Using existing: $GuiDir\dist\app"
}
Write-Host ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 3 — Bundle Python API
# ─────────────────────────────────────────────────────────────────────────────
if (-not $SkipPython) {
    Write-Bold "Phase 3/4 - Bundling Python API"
    Write-Info "Staging Python files into api-bundle\..."

    # Stop local API on 8002 if it holds a lock on api-bundle (desktop app / test uvicorn).
    try {
        $port8002 = Get-NetTCPConnection -LocalPort 8002 -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique
        foreach ($procId in $port8002) {
            if ($procId -and $procId -gt 0) {
                Write-Info "Stopping process on port 8002 (PID $procId) so api-bundle can be updated..."
                Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
            }
        }
        if ($port8002) { Start-Sleep -Seconds 1 }
    } catch {
        # Get-NetTCPConnection may be unavailable; build continues with copy-over fallback.
    }

    if (Test-Path $ApiBundleDir) {
        try {
            Remove-Item -Recurse -Force $ApiBundleDir
        } catch {
            Write-Warn "api-bundle is locked; syncing files in place (close CraftifAI app if copy fails)."
        }
    }
    if (-not (Test-Path $ApiBundleDir)) {
        New-Item -ItemType Directory -Path $ApiBundleDir -Force | Out-Null
    }

    Copy-Item (Join-Path $RepoRoot "refactapi.py")     $ApiBundleDir -Force
    Copy-Item (Join-Path $RepoRoot "requirements.txt") $ApiBundleDir -Force
    Copy-Item (Join-Path $RepoRoot "caps.json")        $ApiBundleDir -Force
    Copy-Item (Join-Path $RepoRoot "file_parsers")     $ApiBundleDir -Recurse -Force
    Copy-Item (Join-Path $RepoRoot "board_definitions") $ApiBundleDir -Recurse -Force
    New-Item -ItemType Directory -Path (Join-Path $ApiBundleDir "configs") -Force | Out-Null
    Copy-Item (Join-Path $RepoRoot "configs\esp32_tools.yaml") (Join-Path $ApiBundleDir "configs\") -Force

    Write-Ok "Python source staged at $ApiBundleDir\"
} else {
    Write-Bold "Phase 3/4 - Skipping Python bundle (-SkipPython)"
    if (-not (Test-Path $ApiBundleDir)) {
        Write-Err "api-bundle\ not found. Run without -SkipPython first."
        exit 1
    }
    Write-Ok "Using existing: $ApiBundleDir"
}
Write-Host ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 4 — Build Electron + package
# ─────────────────────────────────────────────────────────────────────────────
Write-Bold "Phase 4/4 - Building Electron app"
$electronOut = Resolve-ElectronOutputDir $DesktopDir $RepoRoot
$electronOutDir = $electronOut.BuildDir
$electronDefaultOut = $electronOut.DefaultOut
$electronOutArgs = @("--config.directories.output=$electronOutDir")
Push-Location $DesktopDir

if (-not (Test-Path "node_modules")) {
    Write-Info "Installing Electron dependencies..."
    npm ci
    if ($LASTEXITCODE -ne 0) { Write-Err "npm ci failed"; exit 1 }
}

$env:CSC_IDENTITY_AUTO_DISCOVERY = "false"

if ($NsisOnly) {
    Write-Info "Packaging as NSIS installer only..."
    npx electron-builder --win nsis @electronOutArgs
} else {
    Write-Info "Packaging as NSIS + portable..."
    npx electron-builder --win nsis portable @electronOutArgs
}
if ($LASTEXITCODE -ne 0) { Write-Err "electron-builder failed"; exit 1 }

Pop-Location

Copy-ElectronPackagesToDefault $electronOutDir $electronDefaultOut

Write-Host ""
Write-Bold "============================================"
Write-Ok "Build complete!"
Write-Host ""
Write-Host "  Output packages:"
Get-ChildItem (Join-Path $electronDefaultOut "*.exe") -ErrorAction SilentlyContinue |
    ForEach-Object { Write-Host ("    {0} ({1:N1} MB)" -f $_.FullName, ($_.Length / 1MB)) }
if ($electronOutDir -ne $electronDefaultOut) {
    Write-Host ""
    Write-Host "  Intermediate build artifacts:" -ForegroundColor DarkGray
    Write-Host "    $electronOutDir" -ForegroundColor DarkGray
}
Write-Host ""
Write-Bold "  To run now (without packaging):"
Write-Host "  cd craftifai-desktop && npm run dev"
Write-Host ""
