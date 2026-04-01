# CraftifAI ESP32 Agent — Getting Started

## What you received

```
CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage    (127 MB, Linux x86_64)
```

An AI assistant for ESP32 firmware development. Describe what you want in plain English — it builds, flashes, and debugs your firmware.

---

## Before you run it

**1. Check Python (required)**
```bash
python3 --version          # needs 3.10 or newer
python3 -m venv --help     # must work
```

If `venv` is missing:
```bash
sudo apt install python3-venv    # Ubuntu / Debian
sudo dnf install python3         # Fedora
```

**2. Allow serial port access (required for flashing)**
```bash
sudo usermod -aG dialout $USER
# Log out and back in after this
```

**3. Get an OpenAI API key**
→ https://platform.openai.com/api-keys

**4. (Optional) Install ESP-IDF v5.x for actual build/flash support**
```bash
mkdir -p ~/esp && cd ~/esp
git clone --recursive --branch v5.4 https://github.com/espressif/esp-idf.git
cd esp-idf && ./install.sh all
```

---

## Run it

```bash
chmod +x CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage
./CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage
```

On first launch:
1. Setup wizard — enter your API key, ESP-IDF path, board, workspace
2. App installs Python deps automatically (~1–2 min, one time only)
3. Chat interface opens — start talking to your ESP32!

---

## If something goes wrong

```bash
# Run from terminal to see error messages:
./CraftifAI-ESP32-Agent-1.0.0-x86_64.AppImage --no-sandbox

# Check logs:
cat ~/.local/share/craftifai/logs/craftifai.log
cat ~/.local/share/craftifai/logs/pip-install.log

# Reset settings and redo setup:
rm ~/.config/craftifai/settings.json
```

---

**Full documentation:** see `README.md`
