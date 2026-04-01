#!/bin/bash

# Starts the Refact IDE GUI in embedded mode with the launcher service.
# New flow:
#   1. Launcher starts first
#   2. GUI starts
#   3. User fills setup form in GUI
#   4. Launcher starts refact-lsp using user-provided values

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

LAUNCHER_DIR="/home/ritik/Craftif/CRAFTIF/IIS_agent-main/refact-agent/launcher"
LAUNCHER_PORT="${LAUNCHER_PORT:-8009}"
GUI_PORT="${GUI_PORT:-5173}"

# Adjust this only if your GUI project root is elsewhere
GUI_DIR="$SCRIPT_DIR"

cleanup() {
    echo ""
    echo "Stopping background services..."
    if [ -n "$LAUNCHER_PID" ]; then
        kill "$LAUNCHER_PID" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT INT TERM

echo "=========================================="
echo "Starting Refact IDE GUI with Launcher"
echo "=========================================="

# Check launcher folder
if [ ! -d "$LAUNCHER_DIR" ]; then
    echo "ERROR: launcher folder not found at: $LAUNCHER_DIR"
    exit 1
fi

# Check GUI package.json
if [ ! -f "$GUI_DIR/package.json" ]; then
    echo "ERROR: package.json not found in GUI directory: $GUI_DIR"
    exit 1
fi

# Install GUI deps if needed
if [ ! -d "$GUI_DIR/node_modules" ]; then
    echo "Installing GUI dependencies..."
    npm install
fi

# Install launcher deps if needed
if [ ! -d "$LAUNCHER_DIR/node_modules" ]; then
    echo "Installing launcher dependencies..."
    cd "$LAUNCHER_DIR"
    npm install
    cd "$GUI_DIR"
fi

# Start launcher
echo "Starting launcher on http://127.0.0.1:$LAUNCHER_PORT ..."
cd "$LAUNCHER_DIR"
LAUNCHER_PORT="$LAUNCHER_PORT" npm start &
LAUNCHER_PID=$!
cd "$GUI_DIR"

# Wait for launcher
echo "Waiting for launcher to become ready..."
for i in {1..20}; do
    if curl -s "http://127.0.0.1:$LAUNCHER_PORT/v1/launcher/status" >/dev/null 2>&1; then
        echo "Launcher is ready"
        break
    fi
    sleep 0.5
done

if ! curl -s "http://127.0.0.1:$LAUNCHER_PORT/v1/launcher/status" >/dev/null 2>&1; then
    echo "ERROR: launcher did not start properly"
    exit 1
fi

# Embedded mode only; do not force REFACT_LSP_URL now
export VITE_EMBEDDED_MODE="true"
export VITE_LAUNCHER_URL="http://127.0.0.1:$LAUNCHER_PORT"

echo ""
echo "=========================================="
echo "Launcher URL: http://127.0.0.1:$LAUNCHER_PORT"
echo "GUI URL: http://localhost:$GUI_PORT"
echo "Mode: Embedded bootstrap"
echo ""
echo "Open the GUI and fill the setup form:"
echo "  - Workspace path"
echo "  - VecDB path"
echo "  - ESP-IDF path"
echo "  - Board definition"
echo "  - Platform"
echo ""
echo "The launcher will start refact-lsp after form submit."
echo "Press Ctrl+C to stop"
echo "=========================================="
echo ""

# Start GUI dev server
npm run dev
