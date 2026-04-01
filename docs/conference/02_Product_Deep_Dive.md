# Product Deep Dive: Architecture & Capabilities

## Architecture Overview

The Embedded Agent employs a robust **3-Tier Architecture** designed for modularity, local execution, and hardware interaction.

```mermaid
graph TD
    User[User (Browser)] <-->|HTTP/WS| GUI[React GUI (Port 5173)]
    GUI <-->|JSON-RPC/HTTP| Agent[Rust Agent (refact-lsp) (Port 8486)]
    Agent <-->|HTTP| API[Python API Server (Port 8002)]
    
    subgraph "Local Host / Hardware"
        Agent <-->|Serial/USB| Hardware[ESP32 / C2000 Board]
        Agent <-->|Processes| Tools[ESP-IDF / C2000Ware Tools]
    end
    
    subgraph "Knowledge & Logic"
        API <-->|Search| RAG[Static VecDB (Local)]
        API <-->|Inference| LLM[OpenAI / Anthropic / Local]
    end
```

## Core Components

### 1. Rust Agent Engine (`refact-lsp`)
*   **Role**: The "Brain" and "Hands" of the system.
*   **Responsibilities**:
    *   **LSP Server**: Provides code completion, diagnostics, and workspace symbols.
    *   **Tool Execution**: Directly invokes `idf.py`, `esptool.py`, and other native binaries.
    *   **State Management**: Maintains `ESP32GlobalState` (session context, cached configs).
    *   **Hardware Interface**: Manages serial ports and device detection logic.
    *   *Key Files*: `refact-agent/engine/src/tools/esp32_tools/`, `global_state.rs`.

### 2. Python API Server (`refactapi.py`)
*   **Role**: The "Knowledge Hub" and LLM Proxy.
*   **Responsibilities**:
    *   **RAG System**: Serves embeddings and performs semantic search against the static Vector DB (`.vecdb`).
    *   **LLM Proxy**: Sanitizes and forwards prompt chains to the inference engine.
    *   **Capability Discovery**: Serves `caps.json` to inform the agent of available models and tools.
    *   *Key Files*: `refactapi.py`, `caps.json`, `static/esp32_s3_32n8r.vecdb`.

### 3. React GUI
*   **Role**: The "Cockpit" for the user.
*   **Responsibilities**:
    *   **Chat Interface**: Enhanced chat with tool inputs/outputs.
    *   **Integrated Terminal**: For running manual commands or viewing tool output.
    *   **Workspace Explorer**: File tree and editor.
    *   *Key Files*: `refact-agent/gui/`.

## Typical User Flows

### 1. Setup & Configuration
*   **Boot**: `run-all-docker-esp32.sh` orchestrates the containers.
*   **Config**: The agent reads `~/.cache/refact/esp32_tools.yaml` to locate the ESP-IDF SDK (`idf_path`) and default preferences (`default_target`, `default_baud_rate`).
    *   *Evidence*: `esp32_config.rs` (loading logic), `sample_config.yaml`.

### 2. Project Creation & Development
*   **Prompt**: "Create a Zigbee smart switch project."
*   **Execution**:
    1.  Agent checks `board_definitions` for pin constraints.
    2.  `esp32_project` tool copies the relevant SDK template.
    3.  `esp32_config` modifies `sdkconfig` to enable Zigbee.
    4.  `esp32_component` adds necessary dependencies.

### 3. Build, Flash, Monitor
*   **Action**: User clicks "Run" or asks "Flash to device".
*   **Execution**:
    1.  `esp32_build` runs `idf.py build`, streaming output to the chat.
    2.  If errors occur, `error_parser.rs` identifies the root cause (e.g., `HEAD_TO_HEAD_GAP` linker error).
    3.  `esp32_device` detects the port (e.g., `/dev/ttyACM0`), puts the device in bootloader mode (dtr/rts toggling), and flashes.
    4.  Agent switches to monitor mode to show serial logs.

## Configuration Surface

The system is highly configurable via:

1.  **Global Tool Config** (`esp32_tools.yaml`):
    *   `esp_idf_path`: Path to local SDK.
    *   `projects_path`: Workspace root.
    *   `ota_enabled`: Toggle OTA partition generation.
2.  **Board Definitions** (`*.json`):
    *   Defines `safe_pins` (usable GPIOs), `restricted_pins` (strapping pins, flash/PSRAM pins).
    *   Specifies flash size/mode settings (e.g., `32MB`, `qio`).
    *   *Example*: `board_definitions/esp32-s3-devkitc-1-n32r8v.json`.

## Reliability & Guardrails

*   **Deterministic Board Knowledge**: The agent doesn't "guess" pins. It strictly adheres to `board_definition.rs` schema. It knows GPIO 0 is a strapping pin and warns against using it for generic I/O.
*   **Error Deduplication**: The `ErrorParser` class filters redundant build noise to present only unique, actionable errors to the LLM context.
    *   *Evidence*: `error_parser.rs` -> `deduplicate_errors()`.
*   **Safe Flashing**: `esp32_device` implements checks to ensure the binary fits in the partition and the correct chip type is connected before flashing.

## Extensibility

*   **New Boards**: Drop a new JSON file into `board_definitions/` (e.g., `esp32-c6-custom.json`). The agent automatically picks it up via `esp32_device.rs` -> `fetch_board_definition`.
*   **New Tools**: Implement the `Tool` trait in Rust (`refact-agent/engine/src/tools/`) and register it in `tools_list.rs`.
