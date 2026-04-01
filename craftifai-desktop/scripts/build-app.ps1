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

    if (Test-Path $ApiBundleDir) { Remove-Item -Recurse -Force $ApiBundleDir }
    New-Item -ItemType Directory -Path $ApiBundleDir -Force | Out-Null

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
Push-Location $DesktopDir

if (-not (Test-Path "node_modules")) {
    Write-Info "Installing Electron dependencies..."
    npm ci
    if ($LASTEXITCODE -ne 0) { Write-Err "npm ci failed"; exit 1 }
}

if ($NsisOnly) {
    Write-Info "Packaging as NSIS installer only..."
    npm run build:win-nsis
} else {
    Write-Info "Packaging as NSIS + portable..."
    npm run build:win-all
}

Pop-Location

Write-Host ""
Write-Bold "============================================"
Write-Ok "Build complete!"
Write-Host ""
Write-Host "  Output packages:"
Get-ChildItem (Join-Path $DesktopDir "dist-packages\*.exe") -ErrorAction SilentlyContinue |
    ForEach-Object { Write-Host ("    {0} ({1:N1} MB)" -f $_.Name, ($_.Length / 1MB)) }
Write-Host ""
Write-Bold "  To run now (without packaging):"
Write-Host "  cd craftifai-desktop && npm run dev"
Write-Host ""
