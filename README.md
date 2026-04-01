# CraftifAI IIS Agent - Intelligent Embedded Systems Development Agent

<div align="center">
  <h1>IIS Agent</h1>
  <p><strong>Production-Ready AI Agent for Embedded Systems Development</strong></p>
  <p>Transform embedded firmware development from complex command-line workflows to natural language interactions</p>
</div>

---

## 🎯 Overview

IIS (Intelligent IoT SDK) Agent is an AI-powered development assistant specifically designed for embedded systems. It automates complex embedded development workflows, from project creation and driver configuration to building, flashing, and debugging, all through natural language conversation.

### Current Status

- ✅ **TI C2000 Support**: Full production-ready tools for C2000 microcontrollers
- ✅ **ESP32 Support (Beta)**: End-to-end ESP32/ESP-IDF workflow tools (ESP32-P4, ESP32-S3, ESP32-C3, ESP32-C6, and more)
- 🔮 **Future**: Multi-device support (STM32, Nordic, and more)

---

## 🚀 Key Features

### P0 - Core Capabilities (Current & In Development)

#### 1. **Project & Driver Generator**
- Create embedded projects from templates and examples
- Auto-configure pins, clocks, and basic FreeRTOS tasks
- Generate working ESP-IDF/CCS projects with selected peripherals (I2C, SPI, UART, BLE, Wi-Fi, ADC, etc.)

#### 2. **Connectivity & Cloud Templates**
- Ready-made examples for MQTT, HTTP, WebSockets
- Integration templates for AWS IoT, Azure, GCP
- Pre-configured connectivity workflows

#### 3. **OTA Update Framework**
- Unified OTA update pipeline for ESP32
- Partition table management
- Rollback mechanisms

#### 4. **Debug & Diagnostic Scaffolding**
- Intelligent logging and monitoring
- Error code analysis
- Real-time debugging workflows
- Hardware detection and validation

### P1 - Advanced Features (Planned)

- **Cross-MCU Abstraction Layer**: Design for future multi-device support
- **Security & Provisioning**: Secure boot, OTA security, credential management

---

## 🛠️ Supported Platforms

### Currently Supported
- **TI C2000** (F28P65x, F28004x, F28002x, and more)
  - Project creation from C2000Ware examples
  - Build and flash workflows
  - UART capture and monitoring
  - SysConfig modification
  - Target detection
  
- **ESP32 Family** (ESP32-P4, ESP32-S3, ESP32-C3, ESP32-C6, etc.)
  - ESP-IDF project generation from templates and board definitions
  - Build, flash, and monitor workflows driven by `idf.py`
  - OTA-aware project layouts and partition schemes
  - Cloud connectivity and networking templates

---

## 🚀 Quick Start without Docker

**Prerequisites:** Python 3, Node.js/npm, Rust (to build `refact-lsp` once), an OpenAI API key, and a static vecdb file (e.g. `esp32_s3_32n8r.vecdb`).

### 1. One-time installation

From the repo root, run the setup script once (creates Python venv, installs API deps, installs GUI deps):

```bash
chmod +x run-esp32.sh
./run-esp32.sh
```

Optional: build `refact-lsp` and place the static vecdb if you have not already:

```bash
# Build refact-lsp (from repo root)
cd refact-agent/engine && cargo build --release && cd ../..

# Put static vecdb in ./static/ or ~/.cache/refact/static/
mkdir -p static
# cp /path/to/esp32_s3_32n8r.vecdb static/
```

### 2. Run API, agent, and GUI (three terminals)

Open **three terminals**. In each command below, replace `/path/to/refact` with your repo path (e.g. `~/IIS_agent`). Set your OpenAI API key once (e.g. `export OPENAI_API_KEY=sk-...`).

**Terminal 1 – API**

```bash
cd /path/to/refact
source .venv/bin/activate
uvicorn refactapi:app --host 127.0.0.1 --port 8002
```

**Terminal 2 – Agent**

```bash
cd /path/to/refact
export OPENAI_API_KEY=sk-...   # if not already set
./refact-agent/engine/target/release/refact-lsp \
  --address-url http://127.0.0.1:8002 \
  --api-key "$OPENAI_API_KEY" \
  --static-vecdb ./static/esp32_s3_32n8r.vecdb \
  --http-port 8486 \
  --platform esp32 \
  --board-definition esp32-s3-devkitc-1-n32r8v \
  --logs-stderr
```

Use your own vecdb path if not in `./static/` (e.g. `~/.cache/refact/static/esp32_s3_32n8r.vecdb`).

**Terminal 3 – GUI**

```bash
cd /path/to/refact/refact-agent/gui
./start-gui.sh
```

Then open **http://localhost:5173** in your browser.

**If `npm install` fails:** Use Node 18 or 20 (`nvm use` in `refact-agent/gui`). Try `npm ci`; if lockfile is out of sync, use `npm install --legacy-peer-deps`. If `postinstall` (patch-package) fails, run `npm install --ignore-scripts` then `npm run postinstall`.

---

## 🐳 Quick Start with Docker

Docker gives you **stable installation and versioning** (Python, Node, and deps inside containers). The agent uses a **host-built refact-lsp binary** (no Rust in Docker), and you run **three terminals** for control.

**Prerequisites:** Docker and Docker Compose, an OpenAI API key, a static vecdb file (e.g. `esp32_s3_32n8r.vecdb`), and a built `refact-lsp` binary (build on host or use a pre-built one).

### 1. One-time setup

From the repo root:

```bash
# Clone the repository
git clone https://github.com/CraftifAI/IIS_agent.git
cd IIS_agent

# Build refact-lsp on the host (no Rust in Docker) and put it in ./bin
cd refact-agent/engine && cargo build --release && cd ../..
mkdir -p bin && cp refact-agent/engine/target/release/refact-lsp bin/

# Put static vecdb in ./static
mkdir -p static
# cp /path/to/esp32_s3_32n8r.vecdb static/

# Put ESP32 config in .cache/refact (API serves /v1/esp32-config from here)
mkdir -p .cache/refact
cp refact-agent/engine/src/tools/esp32_tools/sample_config.yaml .cache/refact/esp32_tools.yaml
# Edit .cache/refact/esp32_tools.yaml if needed (esp_idf_path, projects_path, etc.)
# The agent service sets REFACT_ESP32_CONFIG_URL so ESP32 tools reach the API container (no binary rebuild).

# Set your OpenAI API key (e.g. in .env or export)
export OPENAI_API_KEY=sk-...
```

### 2. Run API, agent, and GUI (three terminals)

Start **API first**, then **agent**, then **GUI**. Replace `IIS_agent` with your repo path if different.


**Terminal 1 – API**

```bash
cd IIS_agent
docker compose -f docker-compose.test.yml up api
```

**Terminal 2 – Agent** (start after API is up)

```bash
cd IIS_agent
docker compose -f docker-compose.test.yml up agent
```

**Terminal 3 – GUI**

```bash
cd IIS_agent
docker compose -f docker-compose.test.yml up gui
```

Then open **http://localhost:5173** in your browser.

- **Update refact-lsp:** replace `./bin/refact-lsp` and restart the agent (no image rebuild).
- **Rebuild API image:** `docker compose -f docker-compose.test.yml build --no-cache api` if you change API code.

### 2b. One command: API + agent (host) + GUI

To run everything with **one script** (API and GUI in Docker, **agent on the host** so ESP32 device detection sees `/dev/ttyUSB*` and `/dev/ttyACM*`):

```bash
# Put OPENAI_API_KEY in .env or export it, then:
./run-all-docker-esp32.sh
```

- Starts API (Docker) → waits for it → sources ESP-IDF (optional) → starts refact-lsp on host → starts GUI (Docker). Open **http://localhost:5173**. **Ctrl+C** stops all and cleans up.
- Override paths via env: `IDF_EXPORT_SH`, `LSP_BIN`, `STATIC_VECDB`, `WORKSPACE_FOLDER` (see script header).

### 3. Testing & workspace (optional)

**Quick health check** (API + agent + GUI responding):

```bash
./test-endpoints.sh
```

**Agent chat & workspace** (confirms the binary handles chat and workspace):

```bash
./test-agent.sh
```

**GUI workspace in Docker:** The agent only sees **`/workspace`** (mounted from host `./workspace`). In the GUI, set workspace to **`/workspace`** or e.g. `/workspace/myproject`. Put projects under `./workspace/` on the host. The GUI runs in embedded mode so you get the same “Local Workspace” flow as `./start-gui.sh`.

---

### Alternative: Development container (single container)

Use the **default** `docker-compose.yml` + `./start.sh` for one container with Rust, Python, and Node. Enter with `docker compose exec refact-agent bash`, then run API, agent, and GUI manually (see `CONTRIBUTING.md` or the three-terminal commands in “Quick Start without Docker”). Ports 8001, 8002, 8008 are exposed.

### What Docker Provides

- ✅ Python 3 + dependencies (API server in container)
- ✅ Node.js & npm (GUI in container; no local version mismatch)
- ✅ Agent runs your host-built refact-lsp binary (no Rust in Docker; replace `./bin/refact-lsp` to update)
- ✅ Reproducible environment; no manual venv/npm install on the host

---

## 💻 Development

From the repo root: `docker compose up -d`, then `docker compose exec refact-agent bash`. Code is mounted; edit on the host and rebuild inside the container (`cd refact-agent/engine && cargo build`). See **Quick Start without Docker** for the three commands to run API, agent, and GUI.

### Branches

```bash
# Switch to ESP32 development branch
git checkout esp32

# Make changes, commit frequently
git add .
git commit -m "Add ESP32 project create tool"
git push origin esp32
```

---

## 📋 Current Tools

### TI C2000 Tools (Production Ready)

1. **c2000_project_create** - Create CCS projects from C2000Ware examples
2. **c2000_build** - Build projects with various configurations
3. **c2000_flash** - Flash programs to target hardware
4. **c2000_uart_capture** - Capture and analyze UART output
5. **c2000_target_detect** - Detect and verify hardware connections
6. **c2000_example_list** - List available C2000Ware examples
7. **c2000_config_validate** - Validate development environment
8. **c2000_sysconfig_modify** - Modify .syscfg files programmatically
9. **c2000_code_evaluator** - AI-powered code analysis

### ESP32 Tools (Beta)

The ESP32 tools are available behind the `--platform esp32` flag and driven by an `esp32_tools.yaml` configuration (see below):

1. **esp32_project** - Create ESP-IDF projects from templates and board definitions (for example ESP32-S3 DevKitC-1, DevKitM-1)
2. **esp32_build** - Configure and build ESP32 projects using `idf.py` (release/debug, custom targets, partition schemes)
3. **esp32_device** - Discover and manage connected ESP32 boards, serial ports, and basic flashing/monitoring
4. **esp32_config** - Inspect and update the global ESP32 tools configuration (`esp32_tools.yaml`)
5. **esp32_component** - Add or update components (Wi‑Fi, MQTT, peripherals, etc.) in an existing ESP-IDF project
6. **esp32_analyze** - Analyze ESP-IDF build/flash logs and suggest fixes or configuration changes

> Note: ESP32 support is actively evolving. Expect APIs and behaviors to change while we iterate.

---

## 🎯 Goals & Roadmap

### Short-Term (Current Focus)
- ✅ Complete ESP32-P4 tool implementation
- ✅ Generalize tools for all ESP32 variants
- ✅ Production-ready OTA update framework
- ✅ Cloud connectivity templates

### Long-Term Vision
- Multi-device support (STM32, Nordic, etc.)
- Cross-platform abstraction layer
- Advanced security features
- Enterprise-grade deployment tools

---

## ⚙️ Configuring ESP32 Tools

ESP32 tools are configured via a YAML file. A sample file is provided at:

- `refact-agent/engine/src/tools/esp32_tools/sample_config.yaml`

To enable ESP32 workflows:

```bash
mkdir -p ~/.cache/refact
cp refact-agent/engine/src/tools/esp32_tools/sample_config.yaml ~/.cache/refact/esp32_tools.yaml
```

Then edit `~/.cache/refact/esp32_tools.yaml`:

- **esp_idf_path**: Path to your ESP-IDF installation (your `IDF_PATH`)
- **projects_path**: Where new ESP32 projects should be created
- **default_target**: `esp32`, `esp32s3`, `esp32c3`, `esp32c6`, or `esp32p4`
- **default_serial_port** / **default_baud_rate**: Serial connection defaults
- **ota_enabled** / **ota_partition_scheme**: OTA behavior and partition layout
- **cloud_provider** / **mqtt_broker**: Default cloud/MQTT settings (optional)

Board-specific metadata (for example, for ESP32-S3 DevKitC-1 and DevKitM-1) is stored under the repository root in JSON board definition files and used by the ESP32 project generator.

---

## 🏗️ Architecture

- **Backend**: Rust-based LSP server with native embedded tools
- **Frontend**: React-based web GUI
- **Server**: Python-based API server
- **Tools**: Device-specific native tools (compiled into binary)

---

## 📚 Documentation

- **Docker (ESP32):** `docker-compose.test.yml` — see Quick Start with Docker.
- **C2000 Tools:** `refact-agent/engine/src/tools/c2000_tools/README.md`
- **Development:** `CONTRIBUTING.md`

---

## 🤝 Contributing

We welcome contributions! 

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Commit (`git commit -m 'Add amazing feature'`)
5. Push (`git push origin feature/amazing-feature`)
6. Open a Pull Request

---

## 📝 License

See `LICENSE` file for details.

---

## 🙏 Acknowledgments

Built on top of the [Refact.ai](https://github.com/smallcloudai/refact) agent framework, extended with specialized embedded systems capabilities.

---

**Made with ❤️ by CraftifAI for the embedded systems community.**
