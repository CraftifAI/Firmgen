#!/bin/bash
# Post-install script for .deb package
# Installs Python dependencies if the API binary isn't bundled

API_DIR="/opt/CraftifAI ESP32 Agent/resources/api"

if [ -f "$API_DIR/requirements.txt" ] && [ ! -f "$API_DIR/refact-api" ]; then
  echo "Installing Python dependencies for CraftifAI API…"
  pip3 install --quiet -r "$API_DIR/requirements.txt" 2>/dev/null || \
    echo "Warning: Could not install Python deps automatically. Run: pip3 install -r \"$API_DIR/requirements.txt\""
fi
