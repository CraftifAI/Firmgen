#!/usr/bin/env bash
# Quick test that API and agent endpoints respond. Run with all three containers up.
# Usage: ./test-endpoints.sh

set -e

echo "Testing API (port 8002)..."
code_caps=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8002/refact-caps 2>/dev/null || echo "000")
code_esp32=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8002/v1/esp32-config 2>/dev/null || echo "000")

if [ "$code_caps" = "200" ]; then
  echo "  GET /refact-caps     -> $code_caps OK"
else
  echo "  GET /refact-caps     -> $code_caps (expected 200; is API container running?)"
fi
if [ "$code_esp32" = "200" ]; then
  echo "  GET /v1/esp32-config -> $code_esp32 OK"
else
  echo "  GET /v1/esp32-config -> $code_esp32 (expected 200; rebuild API image if still 404)"
fi

echo "Testing Agent (port 8486)..."
code_ping=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8486/v1/ping 2>/dev/null || echo "000")
if [ "$code_ping" = "200" ]; then
  echo "  GET /v1/ping         -> $code_ping OK"
else
  echo "  GET /v1/ping         -> $code_ping (expected 200; is agent container running?)"
fi

echo ""
if [ "$code_caps" = "200" ] && [ "$code_esp32" = "200" ] && [ "$code_ping" = "200" ]; then
  echo "All endpoints OK. Open http://localhost:5173 for the GUI."
else
  echo "Some checks failed. Start API first, then agent, then GUI (see README)."
  exit 1
fi
