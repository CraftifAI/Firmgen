#!/usr/bin/env bash
# Test that the agent (refact-lsp on port 8486) is running and responding to chat
# and workspace (lsp-initialize) requests. Run with API and agent containers up.
# Usage: ./test-agent.sh

set -e

AGENT_URL="${AGENT_URL:-http://localhost:8486}"

echo "Testing agent at $AGENT_URL..."

# 1. Ping
code=$(curl -s -o /dev/null -w "%{http_code}" "$AGENT_URL/v1/ping" 2>/dev/null || echo "000")
if [ "$code" != "200" ]; then
  echo "  GET /v1/ping -> $code (expected 200). Is the agent container running?"
  exit 1
fi
echo "  GET /v1/ping -> 200 OK"

# 2. Caps (agent fetches from API; confirms agent can talk to API)
code=$(curl -s -o /dev/null -w "%{http_code}" "$AGENT_URL/v1/caps" 2>/dev/null || echo "000")
if [ "$code" != "200" ]; then
  echo "  GET /v1/caps -> $code (expected 200). Is the API container up?"
  exit 1
fi
echo "  GET /v1/caps -> 200 OK"

# 3. Set workspace (so chat has a workspace; use /workspace for Docker)
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$AGENT_URL/v1/lsp-initialize" \
  -H "Content-Type: application/json" \
  -d '{"project_roots":["file:///workspace"]}' 2>/dev/null || echo "000")
if [ "$code" != "200" ]; then
  echo "  POST /v1/lsp-initialize -> $code (expected 200). Agent may reject path if not in container."
  exit 1
fi
echo "  POST /v1/lsp-initialize (workspace=file:///workspace) -> 200 OK"

# 4. Minimal chat request (no stream); agent will call API for completion
# Uses minimal body; model must exist in caps (e.g. gpt-4o-mini or code_completion_default_model)
json='{"messages":[{"role":"user","content":"Say exactly: test ok"}],"model":"gpt-4o-mini","stream":false,"meta":{"chat_mode":"EXPLORE"}}'
code=$(curl -s -o /tmp/agent-chat-out.json -w "%{http_code}" -X POST "$AGENT_URL/v1/chat" \
  -H "Content-Type: application/json" \
  -d "$json" 2>/dev/null || echo "000")
if [ "$code" != "200" ]; then
  echo "  POST /v1/chat -> $code (expected 200). Check /tmp/agent-chat-out.json and API key."
  exit 1
fi
echo "  POST /v1/chat -> 200 OK (agent is responding to chat)"

echo ""
echo "Agent is running and responding. Use workspace path /workspace in the GUI (Docker)."
