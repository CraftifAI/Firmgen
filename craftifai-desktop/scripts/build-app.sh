#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# CraftifAI ESP32 Agent — Full Desktop Build Script
#
# Builds everything and produces:
#   craftifai-desktop/dist-packages/CraftifAI-ESP32-Agent-*.AppImage
#   craftifai-desktop/dist-packages/craftifai-esp32-agent_*.deb
#
# Prerequisites:
#   - Rust + cargo   (for refact-lsp)
#   - Node >= 18     (for GUI + Electron)
#   - Python >= 3.10 (for API bundle)
#   - pip install pyinstaller  (for bundling Python API)
#
# Usage:
#   cd craftifai-desktop
#   bash scripts/build-app.sh [--skip-rust] [--skip-gui] [--skip-python] [--appimage-only]
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

# ── Parse flags ───────────────────────────────────────────────────────────────
SKIP_RUST=false
SKIP_GUI=false
SKIP_PYTHON=false
APPIMAGE_ONLY=false

for arg in "$@"; do
  case "$arg" in
    --skip-rust)       SKIP_RUST=true ;;
    --skip-gui)        SKIP_GUI=true ;;
    --skip-python)     SKIP_PYTHON=true ;;
    --appimage-only)   APPIMAGE_ONLY=true ;;
  esac
done

# ── Paths ─────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP_DIR="$(dirname "$SCRIPT_DIR")"        # craftifai-desktop/
REPO_ROOT="$(dirname "$DESKTOP_DIR")"         # repo root

GUI_DIR="$REPO_ROOT/refact-agent/gui"
ENGINE_DIR="$REPO_ROOT/refact-agent/engine"
BIN_DIR="$REPO_ROOT/bin"
API_BUNDLE_DIR="$REPO_ROOT/api-bundle"        # staging dir for Python API

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
info() { printf '  \033[34m→\033[0m %s\n' "$*"; }
ok()   { printf '  \033[32m✓\033[0m %s\n' "$*"; }
err()  { printf '  \033[31m✗\033[0m %s\n' "$*"; }

bold "════════════════════════════════════════════"
bold " CraftifAI ESP32 Agent — Desktop Build"
bold "════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 1 — Build refact-lsp (Rust)
# ─────────────────────────────────────────────────────────────────────────────
if [ "$SKIP_RUST" = false ]; then
  bold "Phase 1/4 · Building refact-lsp (Rust)"
  info "Running: cargo build --release"
  cd "$ENGINE_DIR"
  cargo build --release
  mkdir -p "$BIN_DIR"
  cp target/release/refact-lsp "$BIN_DIR/refact-lsp"
  chmod +x "$BIN_DIR/refact-lsp"
  ok "refact-lsp → $BIN_DIR/refact-lsp  ($(du -sh "$BIN_DIR/refact-lsp" | cut -f1))"
  cd "$REPO_ROOT"
else
  bold "Phase 1/4 · Skipping Rust build (--skip-rust)"
  if [ ! -x "$BIN_DIR/refact-lsp" ]; then
    err "refact-lsp not found at $BIN_DIR/refact-lsp"
    echo "      Build it with: cd refact-agent/engine && cargo build --release && cp target/release/refact-lsp ../../bin/"
    exit 1
  fi
  ok "Using existing: $BIN_DIR/refact-lsp"
fi
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 2 — Build GUI (React → static files)
# ─────────────────────────────────────────────────────────────────────────────
if [ "$SKIP_GUI" = false ]; then
  bold "Phase 2/4 · Building GUI (React → static)"
  cd "$GUI_DIR"

  if [ ! -d node_modules ]; then
    info "Installing npm dependencies…"
    npm ci --prefer-offline 2>&1 | tail -3
  fi

  info "Building standalone app bundle (vite.app.config.ts)…"
  VITE_REFACT_LSP_URL=http://127.0.0.1:8486 \
  VITE_UPLOAD_API_URL=http://127.0.0.1:8002 \
  VITE_EMBEDDED_MODE=true \
    npx vite build --config vite.app.config.ts

  ok "GUI → $GUI_DIR/dist/app/  ($(du -sh "$GUI_DIR/dist/app" | cut -f1))"
  cd "$REPO_ROOT"
else
  bold "Phase 2/4 · Skipping GUI build (--skip-gui)"
  if [ ! -d "$GUI_DIR/dist/app" ]; then
    err "GUI dist not found at $GUI_DIR/dist/app"
    exit 1
  fi
  ok "Using existing: $GUI_DIR/dist/app"
fi
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 3 — Bundle Python API
# ─────────────────────────────────────────────────────────────────────────────
if [ "$SKIP_PYTHON" = false ]; then
  bold "Phase 3/4 · Bundling Python API"

  # Stage all Python API files into api-bundle/
  info "Staging Python files into api-bundle/…"
  rm -rf "$API_BUNDLE_DIR"
  mkdir -p "$API_BUNDLE_DIR"

  cp "$REPO_ROOT/refactapi.py"      "$API_BUNDLE_DIR/"
  cp "$REPO_ROOT/requirements.txt"  "$API_BUNDLE_DIR/"
  cp "$REPO_ROOT/caps.json"         "$API_BUNDLE_DIR/"
  cp -r "$REPO_ROOT/file_parsers"   "$API_BUNDLE_DIR/"
  cp -r "$REPO_ROOT/board_definitions" "$API_BUNDLE_DIR/"
  mkdir -p "$API_BUNDLE_DIR/configs"
  cp "$REPO_ROOT/configs/esp32_tools.yaml" "$API_BUNDLE_DIR/configs/"

  # Try PyInstaller for a zero-dependency binary
  if command -v pyinstaller &>/dev/null; then
    info "PyInstaller found — creating standalone binary…"
    cd "$API_BUNDLE_DIR"

    # Install deps into a local venv for PyInstaller to pick up
    python3 -m venv .venv
    # shellcheck source=/dev/null
    source .venv/bin/activate
    pip install --quiet -r requirements.txt

    pyinstaller \
      --onefile \
      --name refact-api \
      --add-data "caps.json:." \
      --add-data "file_parsers:file_parsers" \
      --add-data "board_definitions:board_definitions" \
      --add-data "configs:configs" \
      --hidden-import uvicorn.logging \
      --hidden-import uvicorn.loops \
      --hidden-import uvicorn.loops.auto \
      --hidden-import uvicorn.protocols \
      --hidden-import uvicorn.protocols.http \
      --hidden-import uvicorn.protocols.http.auto \
      --hidden-import uvicorn.protocols.websockets \
      --hidden-import uvicorn.protocols.websockets.auto \
      --hidden-import uvicorn.lifespan \
      --hidden-import uvicorn.lifespan.on \
      --hidden-import fitz \
      refactapi.py

    cp dist/refact-api "$API_BUNDLE_DIR/refact-api"
    deactivate
    rm -rf build dist __pycache__ .venv *.spec

    ok "Python API → $API_BUNDLE_DIR/refact-api  ($(du -sh "$API_BUNDLE_DIR/refact-api" | cut -f1))"
    cd "$REPO_ROOT"
  else
    info "PyInstaller not found — shipping Python source (requires python3 on target machine)"
    info "Install PyInstaller with: pip install pyinstaller"
    ok "Python source staged at $API_BUNDLE_DIR/"
  fi
else
  bold "Phase 3/4 · Skipping Python bundle (--skip-python)"
  if [ ! -d "$API_BUNDLE_DIR" ]; then
    err "api-bundle/ not found. Run without --skip-python first."
    exit 1
  fi
  ok "Using existing: $API_BUNDLE_DIR"
fi
echo ""

# ─────────────────────────────────────────────────────────────────────────────
# Phase 4 — Build Electron + package
# ─────────────────────────────────────────────────────────────────────────────
bold "Phase 4/4 · Building Electron app"
cd "$DESKTOP_DIR"

if [ ! -d node_modules ]; then
  info "Installing Electron dependencies…"
  npm ci 2>&1 | tail -5
fi

if [ "$APPIMAGE_ONLY" = true ]; then
  info "Packaging as AppImage only…"
  npm run build:appimage
else
  info "Packaging as AppImage + deb…"
  npm run build:all
fi

echo ""
bold "════════════════════════════════════════════"
ok  "Build complete!"
echo ""
echo "  Output packages:"
ls -lh "$DESKTOP_DIR/dist-packages/"*.AppImage 2>/dev/null | awk '{print "    " $NF " (" $5 ")"}' || true
ls -lh "$DESKTOP_DIR/dist-packages/"*.deb       2>/dev/null | awk '{print "    " $NF " (" $5 ")"}' || true
echo ""
bold "  To run now (without packaging):"
echo "  cd craftifai-desktop && npm run dev"
echo ""
