#!/bin/bash

# Helper script to start the Refact IDE GUI
# Auto-detects the refact agent port and starts the GUI

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Function to detect refact agent port
detect_agent_port() {
    # First, try to get port from process list (faster)
    if command -v ps >/dev/null 2>&1; then
        PORT=$(ps aux | grep -E 'refact-lsp|refact.*--http-port' | grep -o '--http-port=[0-9]*' | head -1 | cut -d= -f2)
        if [ -n "$PORT" ]; then
            # Verify it's actually responding
            if curl -s --max-time 0.5 "http://127.0.0.1:$PORT/v1/ping" >/dev/null 2>&1; then
                echo "$PORT"
                return 0
            fi
        fi
    fi
    
    # Fallback: try common ports sequentially
    for port in 8001 $(seq 8100 1 9110); do
        if curl -s --max-time 0.3 "http://127.0.0.1:$port/v1/ping" >/dev/null 2>&1; then
            echo "$port"
            return 0
        fi
    done
    
    return 1
}

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
fi

# Detect agent port
echo "Detecting refact agent port..."
AGENT_PORT=$(detect_agent_port)
if [ $? -ne 0 ] || [ -z "$AGENT_PORT" ]; then
    echo "ERROR: Could not detect refact agent port"
    echo "Make sure the refact agent is running with: refact <workspace_path>"
    exit 1
fi
echo "Found agent on port $AGENT_PORT"

# Set environment variables
export REFACT_LSP_URL="http://127.0.0.1:$AGENT_PORT"
export VITE_REFACT_LSP_URL="http://127.0.0.1:$AGENT_PORT"
export REFACT_LSP_PORT="$AGENT_PORT"
export VITE_EMBEDDED_MODE="true"


echo ""
echo "=========================================="
echo "Starting Refact IDE GUI"
echo "=========================================="
echo "Agent URL: $REFACT_LSP_URL"
echo "Agent Port: $AGENT_PORT"
echo "GUI will be available at: http://localhost:5173"
echo ""
echo "Press Ctrl+C to stop"
echo "=========================================="
echo ""

# Start the dev server
npm run dev

