#!/usr/bin/env bash
# One-time installation for ESP32 flow (API + Agent + GUI).
# Run this once; then use three separate terminals to start API, agent, and GUI
# (see README "Quick Start without Docker" for the three commands).
#
# Usage: ./run-esp32.sh
# Optional: SKIP_SETUP=1 to skip steps that are already done (e.g. re-run only npm).

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

echo "=== One-time setup (API + GUI deps) ==="

# Python venv + API deps
if [ ! -d ".venv" ]; then
  echo "Creating Python venv..."
  python3 -m venv .venv
fi
echo "Installing Python API deps..."
# shellcheck disable=SC1091
source .venv/bin/activate
pip install -q -r requirements.txt

# GUI deps
if [ ! -d "refact-agent/gui/node_modules" ]; then
  echo "Installing GUI deps (npm install --legacy-peer-deps)..."
  (cd refact-agent/gui && npm install --legacy-peer-deps)
else
  echo "GUI node_modules present; skipping npm install. To reinstall: cd refact-agent/gui && npm install --legacy-peer-deps"
fi

echo "Setup done. Use three terminals to run API, agent, and GUI (see README)."
