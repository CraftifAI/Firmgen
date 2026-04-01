# How to Start the Refact Agent

## Understanding the Port

The refact agent can run on different ports:

1. **Default port**: 8001 (if you specify `--http-port 8001`)
2. **Random port**: 8100-9100 (default when using `refact` CLI command)
3. **Custom port**: Any port you specify

## Method 1: Start Agent on Port 8001 (Recommended)

To start the agent on a specific port (8001), you need to find the refact-lsp binary and run it directly:

```bash
# Find the refact-lsp binary
which refact-lsp
# Or it might be in:
# ~/.cache/refact/refact-lsp
# Or in the refact-agent/engine directory

# Start it on port 8001
refact-lsp --http-port 8001 --logs-stderr <workspace_path>
```

However, the easier way is to use the Python CLI with environment variable:

```bash
# Set the port via environment variable (if supported)
# Or modify the CLI to pass --http-port

# Actually, the simplest way:
refact <workspace_path> --http-port 8001
```

Wait, let me check if the CLI supports this...

## Method 2: Use the Python CLI (Auto-detects port)

The Web UI now **auto-detects** the agent port! So you can just:

```bash
# Start the agent normally
refact /path/to/workspace

# The agent will start on a random port (8100-9100)
# The Web UI will automatically find it!
```

The Web UI will try to connect to:
1. Port 8001 (default)
2. Ports 8100-9100 (auto-detect)

## Method 3: Configure Port in Web UI

You can also set the port via environment variable:

```bash
# Set the expected port
export REFACT_AGENT_PORT=8001

# Start the Web UI
python -m refact_webgui.webgui.webgui
```

## Method 4: Check What Port the Agent is Using

When you start the agent with `refact`, it will print:
```
HTTP server listening on 127.0.0.1:XXXX
```

Look for this line in the terminal output to see what port it's using.

## Quick Start Guide

### Step 1: Start the Agent

```bash
# Navigate to your workspace
cd /path/to/your/workspace

# Start the agent (it will use a random port)
refact .
```

**Look for this output:**
```
HTTP server listening on 127.0.0.1:8234
```
(Note the port number - it will be different each time)

### Step 2: Start the Web UI

```bash
cd refact-server
python -m refact_webgui.webgui.webgui
```

### Step 3: Use the Chat Tab

1. Open http://127.0.0.1:8008
2. Click "Chat" tab
3. Click "Check Connection" button
4. The Web UI will automatically detect the agent port!

## Troubleshooting

### "Could not connect to refact agent"

1. **Make sure the agent is running:**
   ```bash
   # Check if agent is running
   ps aux | grep refact-lsp
   ```

2. **Check what port the agent is using:**
   ```bash
   # Look at the agent terminal output for:
   # "HTTP server listening on 127.0.0.1:XXXX"
   ```

3. **Test the agent directly:**
   ```bash
   # Try common ports
   curl http://127.0.0.1:8001/v1/ping
   curl http://127.0.0.1:8100/v1/ping
   curl http://127.0.0.1:8200/v1/ping
   # etc.
   ```

4. **Set the port manually:**
   ```bash
   export REFACT_AGENT_PORT=XXXX  # Use the port from step 2
   python -m refact_webgui.webgui.webgui
   ```

### Agent Port Changes Each Time

This is normal! The `refact` CLI uses random ports. The Web UI will auto-detect it, but if you want a fixed port:

1. Find the refact-lsp binary location
2. Run it directly with `--http-port 8001`
3. Or set `REFACT_AGENT_PORT=8001` environment variable

## Summary

- **Easiest**: Just run `refact <workspace>` - the Web UI will auto-detect the port
- **Fixed port**: Set `REFACT_AGENT_PORT` environment variable
- **Check port**: Look for "HTTP server listening" in agent terminal output










