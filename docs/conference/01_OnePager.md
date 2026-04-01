# ESP32 Tools / Embedded Agent: One-Pager

## Elevator Pitch
The **Intelligent Embedded Systems Development Agent** transforms complex command-line embedded workflows into seamless natural language interactions. Built on a robust 3-tier architecture (Rust Agent, Python API, React GUI), it automates the entire development lifecycle—from project creation and configuration to building, flashing, and debugging—specifically optimized for **ESP32** (ESP-IDF) and **TI C2000** platforms. It acts as an expert pair programmer that understands hardware specifics, SDK constraints, and build errors.

## Who It's For
*   **Embedded Firmware Engineers** seeking to reduce boilerplate and configuration fatigue.
*   **Hardware Product Teams** needing standardized, reproducible build/flash environments.
*   **Developers** transitioning to ESP-IDF or TI C2000 ecosystems who want intelligent guidance.

## Key Benefits
1.  **Hardware-Aware Automation**: Unlike generic coding assistants, this agent understands *your* specific hardware. It uses board definition files (e.g., `esp32-s3-devkitc-1-n32r8v.json`) to auto-configure pins, memory, and peripherals correctly from day one.
    *   *Evidence*: `board_definitions/` directory, `esp32_project.rs`
2.  **End-to-End Workflow Integration**: It doesn't just write code; it drives the tools. The agent can compile code, analyze build sizes, flash generation-generated binaries to the device, and monitor serial output—all within the chat interface.
    *   *Evidence*: `esp32_build.rs`, `esp32_device.rs`, `esp32_tools.yaml`
3.  **Local, Privacy-First Architecture**: Designed for air-gapped or secure environments. It uses a local static Vector DB for RAG (Retrieval-Augmented Generation) and runs fully within Docker containers or on your host, ensuring your IP never leaves your control if configured for local LLMs.
    *   *Evidence*: `static/esp32_s3_32n8r.vecdb`, `run-all-docker-esp32.sh`
4.  **Intelligent Error Resolution**: The agent parses GCC and ESP-IDF specific build errors to provide context-aware fix suggestions, going beyond generic syntax highlighting to understand linker and SDK compliance issues.
    *   *Evidence*: `error_parser.rs`, `esp32_analyze.rs`

## Core Capabilities
1.  **Smart Project Generation**: Instantly scaffolds production-ready projects based on official SDK templates and your specific board definition.
    *   *Source*: `tools/esp32_tools/esp32_project.rs`
2.  **Automated Device Management**: Auto-detects connected boards, serial ports, and flash parameters, removing the need for manual `dmesg` or COM port hunting.
    *   *Source*: `tools/esp32_tools/esp32_device.rs`
3.  **Dynamic Configuration**: managing `sdkconfig`, partition tables, and component dependencies through natural language or structured tools.
    *   *Source*: `tools/esp32_tools/esp32_config.rs`
4.  **Integrated RAG Knowledge Base**: Uses a specialized vector database indexed with board-specific documentation, pinouts, and API references.
    *   *Source*: `refactapi.py` (embeddings endpoint), `static/*.vecdb`

## How It Works (6 Steps)
1.  **Install**: Run the standardized deployment script (`./run-all-docker-esp32.sh`) to spin up the Agent, API, and GUI.
2.  **Configure**: Define your environment (IDF paths, default targets) in `esp32_tools.yaml`.
3.  **Connect**: Plug in your ESP32 or C2000 board; the agent auto-detects it via `esp32_device` or `c2000_target_detect`.
4.  **Prompt**: Ask the agent, "Create a Wi-Fi station project for my ESP32-S3 that blinks an LED while connecting."
5.  **Build & Flash**: The agent generates code, runs `idf.py build`, fixes any errors, and flashes the binary (`idf.py flash`).
6.  **Monitor**: View real-time serial logs in the GUI to verify functionality.

## Example Use Cases
*   **"Wi-Fi Station & Status LED"**: The agent generates a project connecting to a specified SSID, implementing a state machine for LED blinking during connection attempts, and handling reconnection logic—all backed by the SDK's station example.
    *   *Reference*: `example_wifi_prompt_esp32_agent.txt`
*   **"TI C2000 SysConfig"**: For TI users, the agent can interact with and modify `SysConfig` setups for complex peripheral configuration on F28P65x chips.
    *   *Reference*: `refactapi.py` (`v1/c2000-sysconfig-recipe`)
