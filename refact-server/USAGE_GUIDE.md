# How to Use the Refact Web UI with Chat Interface

## Architecture Overview

There are **two separate components** that work together:

1. **Refact Agent (Rust-based)** - Runs on port **8001** (or random port 8100-9100)
   - Provides the chat API with C2000 tools
   - Started via CLI: `refact <workspace_path>`
   - This is what you've been using in the terminal

2. **Refact Server Web UI (Python-based)** - Runs on port **8008**
   - Provides the web interface
   - Includes the new Chat tab that connects to the agent
   - Started via: `python -m refact_webgui.webgui.webgui`

## How to Use

### Step 1: Start the Refact Agent

First, start the refact agent (the one with C2000 tools):

```bash
# Navigate to your project directory
cd /path/to/your/project

# Start the refact agent
refact .
```

Or if you have a specific workspace:

```bash
refact /path/to/workspace
```

The agent will start and listen on port **8001** (or a random port between 8100-9100).

**Note:** Keep this terminal open - the agent needs to keep running.

### Step 2: Start the Web UI

In a **separate terminal**, start the Web UI:

```bash
cd refact-server
python -m refact_webgui.webgui.webgui
```

The Web UI will start on port **8008**.

### Step 3: Access the Web UI

Open your browser and go to:

**http://127.0.0.1:8008**

### Step 4: Use the Chat Interface

1. Click on the **"Chat"** tab in the navigation bar
2. The UI will check if the agent is connected (you should see "Connected" badge)
3. If not connected, make sure the agent is running (Step 1)
4. Type your message in the chat input, for example:
   - "Create a SPI loopback project for F28P65x"
   - "Build my project and flash it to the board"
   - "List available C2000Ware examples"
   - "Help me debug this communication issue"
5. Click "Send" or press Enter
6. The agent will respond and can use C2000 tools automatically

## What You Can Do

### In the Chat Tab:
- **Chat with the agent** - Natural language interaction
- **Use C2000 tools** - The agent automatically uses tools like:
  - `c2000_project_create`
  - `c2000_build`
  - `c2000_flash`
  - `c2000_uart_capture`
  - `c2000_target_detect`
  - And more...
- **See available tools** - View all tools the agent can use
- **Stream responses** - Watch responses come in real-time

### In the C2000 Tools Tab:
- **Configure paths** - Set CCS, C2000Ware, workspace paths
- **Manage projects** - List and manage your C2000 projects
- **Browse examples** - Search and browse C2000Ware examples
- **Detect targets** - Check connected hardware
- **Debug assistant** - AI-powered debugging help
- **Log analysis** - Analyze build and debug logs

## Troubleshooting

### "Could not connect to refact agent"

**Solution:** Make sure the refact agent is running:
```bash
refact <workspace_path>
```

### Agent is on a different port

The agent might be on a port other than 8001. Check the agent terminal output - it will show:
```
HTTP server listening on 127.0.0.1:XXXX
```

If it's on a different port, you can modify `tab_chat.py` to use that port, or restart the agent.

### Port 8008 already in use

Use a different port for the Web UI:
```bash
python -m refact_webgui.webgui.webgui --port 8080
```

Then access it at `http://127.0.0.1:8080`

## Key Differences: CLI vs Web UI

### CLI (Terminal):
```bash
refact .
# Then type messages in terminal
```

### Web UI:
1. Start agent: `refact .` (in one terminal)
2. Start Web UI: `python -m refact_webgui.webgui.webgui` (in another terminal)
3. Use browser at http://127.0.0.1:8008
4. Click "Chat" tab
5. Type messages in the web interface

**Both use the same agent and C2000 tools!** The Web UI is just a different interface to the same agent.

## Example Workflow

1. **Start agent:**
   ```bash
   refact /home/shubham/ti/ccs_workspace
   ```

2. **Start Web UI:**
   ```bash
   cd refact-server
   python -m refact_webgui.webgui.webgui
   ```

3. **Open browser:** http://127.0.0.1:8008

4. **Go to Chat tab** and type:
   ```
   Create a SPI loopback project for F28P65x LaunchPad
   ```

5. **Agent responds** and creates the project using `c2000_project_create` tool

6. **Continue chatting:**
   ```
   Build it and flash to the board
   ```

7. **Agent uses** `c2000_build` and `c2000_flash` tools automatically!

## Summary

- **Agent (port 8001)**: The "brain" with C2000 tools - started with `refact` command
- **Web UI (port 8008)**: The interface - started with `python -m refact_webgui.webgui.webgui`
- **Chat Tab**: Connects Web UI to Agent, so you can chat through the browser instead of terminal
- **C2000 Tools Tab**: Direct UI for C2000 configuration and management

You can use **either** the CLI or the Web UI Chat - both connect to the same agent with the same C2000 tools!

