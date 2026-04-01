# CraftifAI ESP32 Agent — Desktop App

AI-powered firmware development for ESP32. Write, build, flash, and debug firmware using natural language — no terminal juggling required.

---

## What's inside

| Component | Description |
|-----------|-------------|
| **Chat UI** | React-based AI chat interface |
| **refact-lsp** | AI agent with ESP32 tools (build, flash, monitor, GPIO, WiFi config, etc.) |
| **refactapi** | Local OpenAI proxy + embeddings + file parser |
| **Auto-venv** | Python dependencies installed locally on first launch |

---

## System Requirements

| Requirement | Minimum | Notes |
|-------------|---------|-------|
| **OS** | Ubuntu 20.04 / Debian 11 / Fedora 35 or newer | x86_64 only |
| **Python** | 3.10 or newer | Check: `python3 --version` |
| **RAM** | 4 GB | 8 GB recommended |
| **Disk** | 1 GB free | ~200 MB for Python deps installed on first run |
| **Internet** | Required on first launch | For Python dep install + OpenAI API calls |
| **OpenAI API key** | Required | [Get one here](https://platform.openai.com/api-keys) |

> **ESP-IDF is optional at install time.** You can add the path later in settings. It is only required for building and flashing firmware.

---

## Quick Start (3 steps)

### Step 1 — Download

Grab the AppImage from the releases page or copy it from the shared location:

```
CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage   (127 MB)
```

### Step 2 — Make executable and run

```bash
chmod +x CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage
./CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage
```

Or double-click the file in your file manager (if your distro supports AppImages).

### Step 3 — Complete the setup wizard

On first launch a setup wizard walks you through:

1. **OpenAI API Key** — paste your `sk-...` key
2. **ESP-IDF Path** — path to `esp-idf/export.sh` (browse or type; can be skipped)
3. **Board selection** — pick your ESP32 board from the list
4. **Workspace folder** — where your firmware projects will live

After that, the app installs its Python dependencies (~1–2 min, one-time only), then opens the chat interface.

---

## Prerequisites in Detail

### Python 3.10+

The app creates a self-contained Python virtual environment in `~/.config/craftifai/venv/` on first launch. It does **not** install anything system-wide.

Check you have a working Python 3:

```bash
python3 --version        # should be 3.10, 3.11, or 3.12
python3 -m venv --help   # must exist
```

If `python3 -m venv` is missing:

```bash
# Ubuntu / Debian
sudo apt install python3-venv python3-pip

# Fedora
sudo dnf install python3

# Arch
sudo pacman -S python
```

### ESP-IDF (for building/flashing firmware)

The app works without ESP-IDF installed — you can still use the AI chat. You need it to actually **build and flash** firmware.

Install ESP-IDF v5.x:

```bash
mkdir -p ~/esp && cd ~/esp
git clone --recursive --branch v5.4 https://github.com/espressif/esp-idf.git
cd esp-idf && ./install.sh all
```

The path you need for the setup wizard is: `~/esp/esp-idf/export.sh`

### Serial / USB access (important for flashing)

Your user must be in the `dialout` group to access `/dev/ttyUSB*` and `/dev/ttyACM*`:

```bash
sudo usermod -aG dialout $USER
# Log out and back in (or run: newgrp dialout)
```

Verify your device is visible after connecting the ESP32:

```bash
ls /dev/ttyUSB* /dev/ttyACM* 2>/dev/null
```

### FUSE (required for AppImage on some distros)

AppImages need FUSE to run. Most distros have it already.

```bash
# Ubuntu / Debian
sudo apt install libfuse2

# Fedora
sudo dnf install fuse

# If FUSE is unavailable, extract and run directly:
./CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage --appimage-extract
./squashfs-root/AppRun
```

---

## File Locations

| Path | Contents |
|------|----------|
| `~/.config/craftifai/settings.json` | API key, IDF path, board, workspace |
| `~/.config/craftifai/venv/` | Python virtual environment (auto-created) |
| `~/.local/share/craftifai/logs/craftifai.log` | Main app log |
| `~/.local/share/craftifai/logs/api.log` | Python API log |
| `~/.local/share/craftifai/logs/lsp.log` | Agent (refact-lsp) log |
| `~/.local/share/craftifai/logs/pip-install.log` | Dep install log (first run) |
| `~/craftifai-workspace/` | Default firmware project workspace |

---

## Troubleshooting

### App won't start / blank window

```bash
# Run from terminal to see errors:
./CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage --no-sandbox
```

### "Python venv creation failed"

Make sure `python3-venv` is installed:

```bash
sudo apt install python3-venv   # Ubuntu/Debian
```

### "pip install failed" on first run

Check the pip log:

```bash
cat ~/.local/share/craftifai/logs/pip-install.log
```

Common fix — missing system build tools:

```bash
sudo apt install python3-dev build-essential
```

### Port already in use (8002 or 8486)

Another instance may be running. Kill it:

```bash
lsof -ti:8002 | xargs kill -9 2>/dev/null
lsof -ti:8486 | xargs kill -9 2>/dev/null
```

### "Permission denied" on serial port

```bash
sudo usermod -aG dialout $USER
# Then log out and back in
```

### Reset and start over

Click **View Logs → Reset Setup** inside the app's error screen, or:

```bash
rm ~/.config/craftifai/settings.json
# Relaunch the app — setup wizard will appear again
```

To also wipe the Python venv (force full reinstall):

```bash
rm -rf ~/.config/craftifai/venv/
```

---

## Updating the App

Simply replace the `.AppImage` file with the new version and relaunch.

- Settings are preserved (stored in `~/.config/craftifai/`)
- If `requirements.txt` changed, the app auto-reinstalls Python deps on next launch

---

## Ports Used

| Port | Service |
|------|---------|
| `8002` | Python API (OpenAI proxy, embeddings, board definitions) |
| `8486` | refact-lsp (AI agent HTTP interface) |

Both ports are localhost-only and not exposed to the network.

---

## Building From Source

If you want to rebuild the app yourself:

```bash
# Prerequisites
# - Rust + cargo
# - Node.js >= 18
# - Python 3.10+
# - pip install pyinstaller  (optional, for a self-contained binary)

git clone <repo-url>
cd refact/craftifai-desktop
bash scripts/build-app.sh
```

Flags:

```bash
bash scripts/build-app.sh --skip-rust      # if refact-lsp is already built
bash scripts/build-app.sh --skip-gui       # if GUI is already built
bash scripts/build-app.sh --appimage-only  # skip .deb output
```

Output: `craftifai-desktop/dist-packages/CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage`

---

## Tested On

| Distro | Version | Status |
|--------|---------|--------|
| Ubuntu | 22.04 LTS | ✅ Verified |
| Ubuntu | 20.04 LTS | ✅ Should work |
| Fedora | 38+ | ✅ Should work (needs `fuse`) |
| Arch Linux | rolling | ✅ Should work (needs `fuse2`) |

---

## License

BSD 3-Clause — see `LICENSE` in the repository root.
