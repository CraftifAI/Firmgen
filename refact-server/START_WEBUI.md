# How to Start the Refact Web UI

## Quick Start

The Web UI can be started in several ways:

### Method 1: Direct Python Module (Recommended for Development)

Navigate to the `refact-server` directory and run:

```bash
cd refact-server
python -m refact_webgui.webgui.webgui
```

Or with custom host/port:

```bash
python -m refact_webgui.webgui.webgui --host 0.0.0.0 --port 8008
```

### Method 2: Using Python Script Directly

```bash
cd refact-server
python refact_webgui/webgui/webgui.py
```

### Method 3: After Installation

If you've installed the package:

```bash
pip install -e refact-server/
python -m refact_webgui.webgui.webgui
```

## Access the Web UI

Once started, open your browser and navigate to:

**http://127.0.0.1:8008**

or

**http://localhost:8008**

## Default Settings

- **Host**: `0.0.0.0` (listens on all interfaces)
- **Port**: `8008`
- **URL**: `http://127.0.0.1:8008`

## Custom Port

To use a different port:

```bash
python -m refact_webgui.webgui.webgui --port 8080
```

## Features Available

Once the Web UI is running, you'll have access to:

- **Model Hosting** - Manage and host AI models
- **Third-Party APIs** - Configure external API keys
- **Stats** - View usage statistics
- **Projects** - Manage projects
- **Finetune** - Fine-tune models
- **C2000 Tools** - C2000 microcontroller development tools (newly added!)
- **Settings** - Configure server settings
- **Server Logs** - View server logs
- **About** - Version information

## Troubleshooting

### Port Already in Use

If port 8008 is already in use, either:
1. Stop the process using that port
2. Use a different port with `--port` flag

### Module Not Found

If you get `ModuleNotFoundError`, make sure you:
1. Are in the correct directory (`refact-server`)
2. Have installed dependencies: `pip install -e .`
3. Are using the correct Python environment

### Database Connection Issues

The Web UI may require a database. If you see database errors:
1. Check if the database service is running
2. Set `REFACT_DATABASE_HOST` environment variable if using external database

## Environment Variables

Optional environment variables:

- `REFACT_ADMIN_TOKEN` - Set admin authentication token
- `REFACT_DATABASE_HOST` - Database host (if not using default)

Example:

```bash
REFACT_ADMIN_TOKEN=your_token_here python -m refact_webgui.webgui.webgui
```










