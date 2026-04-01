#!/usr/bin/env bash
# Run API (Docker) + refact-lsp (host) + GUI (Docker) with one command.
# Agent runs on host so it can see /dev/ttyUSB* and /dev/ttyACM* for ESP32 device detection.
#
# Prerequisites:
#   - Docker; API and GUI images built (docker compose -f docker-compose.test.yml build api gui)
#   - refact-lsp built (see README) and present in bin/ or refact-agent/engine/target/release/
#   - OPENAI_API_KEY in .env or exported
#   - Optional: IDF_EXPORT_SH path to esp-idf/export.sh (default below)
#
# Usage: ./run-all-docker-esp32.sh
# Ctrl+C stops the GUI and cleans up (stops API container, kills refact-lsp).

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# --- Config (override via env) ---
# Path to ESP-IDF export script (sourced so esptool etc. are on PATH)
export IDF_EXPORT_SH="${IDF_EXPORT_SH:-/home/shubham/sdk_agent/ESP_v5.5/esp-idf-release-v5.5/export.sh}"
# refact-lsp binary: prefer bin/refact-lsp, else target/release
LSP_BIN="${LSP_BIN:-}"
if [ -z "$LSP_BIN" ]; then
  if [ -x "$REPO_ROOT/bin/refact-lsp" ]; then
    LSP_BIN="$REPO_ROOT/bin/refact-lsp"
  elif [ -x "$REPO_ROOT/refact-agent/engine/target/release/refact-lsp" ]; then
    LSP_BIN="$REPO_ROOT/refact-agent/engine/target/release/refact-lsp"
  else
    echo "refact-lsp not found. Build with: cd refact-agent/engine && cargo build --release && cp target/release/refact-lsp ../../bin/"
    exit 1
  fi
fi
# Static vecdb: prefer ~/.cache/refact/static, else repo static/
STATIC_VECDB="${STATIC_VECDB:-}"
if [ -z "$STATIC_VECDB" ]; then
  if [ -f "$HOME/.cache/refact/static/K_b.vecdb" ]; then
    STATIC_VECDB="$HOME/.cache/refact/static/K_b.vecdb"
  elif [ -f "$REPO_ROOT/static/K_b.vecdb" ]; then
    STATIC_VECDB="$REPO_ROOT/static/K_b.vecdb"
  else
    echo "Static vecdb not found. Copy K_b.vecdb to static/ or ~/.cache/refact/static/"
    exit 1
  fi
fi
WORKSPACE_FOLDER="${WORKSPACE_FOLDER:-$REPO_ROOT/workspace}"
mkdir -p "$WORKSPACE_FOLDER"

# --- Load API key ---
if [ -f "$REPO_ROOT/.env" ]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi
if [ -z "${OPENAI_API_KEY:-}" ]; then
  echo "Set OPENAI_API_KEY (e.g. in .env or export OPENAI_API_KEY=sk-...)"
  exit 1
fi

# --- Cleanup on exit ---
API_PID=""
LSP_PID=""
cleanup() {
  echo "Stopping services..."
  [ -n "$LSP_PID" ] && kill "$LSP_PID" 2>/dev/null || true
  docker compose -f "$REPO_ROOT/docker-compose.test.yml" down --remove-orphans 2>/dev/null || true
  exit 0
}
trap cleanup EXIT INT TERM

# --- 1. Start API (Docker) in background ---
echo "Starting API (Docker)..."
docker compose -f docker-compose.test.yml up api &
API_PID=$!

# --- 2. Wait for API to be up ---
echo "Waiting for API at http://127.0.0.1:8002..."
until curl -sf http://127.0.0.1:8002/refact-caps >/dev/null 2>&1; do
  sleep 1
done
echo "API is up."

# --- 3. Source ESP-IDF (optional) ---
if [ -z "${IDF_PATH:-}" ] && [ -f "$IDF_EXPORT_SH" ]; then
  echo "Sourcing ESP-IDF: $IDF_EXPORT_SH"
  # shellcheck source=/dev/null
  source "$IDF_EXPORT_SH"
elif [ -n "${IDF_PATH:-}" ]; then
  echo "IDF_PATH already set, skipping export."
else
  echo "IDF_EXPORT_SH not found or empty; continuing without ESP-IDF (device tools may need it)."
fi

# --- 4. Start refact-lsp on host in background ---
echo "Starting refact-lsp on host (port 8486)..."
"$LSP_BIN" \
  --address-url http://127.0.0.1:8002 \
  --api-key "$OPENAI_API_KEY" \
  --ast --ast-max-files 20000 \
  --logs-stderr \
  --http-port=8486 \
  --platform esp32 \
  --board-definition esp32-s3-DevKitM-1-N16R8 \
  --workspace-folder "$WORKSPACE_FOLDER" &
LSP_PID=$!

#--static-vecdb "$STATIC_VECDB" \
# Give LSP a moment to bind
sleep 2

# --- 5. Start GUI (Docker) in foreground ---
echo "Starting GUI (Docker). Open http://localhost:5173 — Ctrl+C to stop all."
docker compose -f docker-compose.test.yml up gui
