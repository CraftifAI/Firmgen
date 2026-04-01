# Competitive Differentiation

## Positioning
We are **not** a general-purpose coding assistant. We are a **Specialized Embedded Systems Agent**.
While general tools optimize for Python/JS web apps, we optimize for **Hardware-in-the-Loop** development, SDK constraints, and binary size management.

## Comparison Matrix

| Feature | **Our Embedded Agent** | **Claude Code** | **Cursor** | **Antigravity** |
| :--- | :--- | :--- | :--- | :--- |
| **Primary Target** | Embedded / Firmware Engineers | General Software Devs | General Software Devs | Agent Developers |
| **Hardware Awareness** | ✅ **Native** (Board Defs, Pinouts) | ❌ None (Hallucinates pins) | ❌ None | ❌ None |
| **Tool Integration** | ✅ **Direct** (IDF, Esptool, OpenOCD) | ⚠️ CLI Wrapper Only | ❌ IDE Plugins Only | ⚠️ Framework Dependent |
| **Offline / Air-gapped** | ✅ **Yes** (Local Docker + VecDB) | ❌ Cloud Only | ⚠️ Partial (Local Models) | ⚠️ Partial |
| **Error Handling** | ✅ **SDK-Specific** (GCC/Linker Parsers) | ⚠️ Generic Text Analysis | ⚠️ Generic Copilot | ⚠️ Generic |
| **Project Context** | ✅ **Deep** (CMake/Kconfig aware) | ⚠️ File-based only | ✅ File-based | ✅ Graph-based |
| **Deterministic Safety** | ✅ **High** (Guardrails on GPIOs) | ❌ Low | ❌ Low | ⚠️ Configurable |

*(Note: Competitor capabilities based on general public knowledge. Our capabilities backed by repo evidence.)*

## Why We Win for ESP32 / Embedded

### 1. The "Hardware Truth" Source
Generic LLMs guess pinouts. We **know** them.
*   **differentiation**: We use strict JSON schemas (`board_definitions/*.json`) to define every pin's capability (PWM, ADC, Strapping).
*   **Evidence**: `esp32_device.rs` validates operations against `esp32-s3-devkitc-1-n32r8v.json` before suggesting code. If a user asks to use GPIO 0 (a strapping pin) for a relay, we warn them.

### 2. Native Toolchain Orchestration
We don't just "suggest" commands; we execute them with state awareness.
*   **differentiation**: Our `esp32_build` tool wraps `idf.py` to capture strict output formats, parse errors via `error_parser.rs`, and feed them back into the context loop automatically.
*   **Evidence**: The agent can run `idf.py menuconfig`, modify `sdkconfig` via `esp32_config`, and re-build without user intervention.

### 3. Local-First Knowledge (RAG)
No internet? No problem.
*   **differentiation**: We ship with a pre-indexed Vector DB (`static/esp32_s3_32n8r.vecdb`) containing the exact documentation for the supported SDK version. No need to scrape the web or verify version mismatches.
*   **Evidence**: `refactapi.py` serves search results from local `.vecdb` files.

### 4. Smart Peripheral Configuration
Configuring peripherals involves complex dependencies (clocks, dma, conflicts).
*   **differentiation**: Our `esp32_config` tool understands these relationships. It can resolve `ADC2` conflicts with Wi-Fi (a common ESP32 pitfall) because logic is hardcoded in `esp32_config.rs`.
*   **Evidence**: `check_conflicts` logic in `esp32_config.rs`.

### 5. Automated "Brick" Prevention
Flashing the wrong binary or partition table can brick a device.
*   **differentiation**: `esp32_device` checks the connected chip type against the project target before allowing a flash operation. It validates partition tables against flash size (e.g., stopping a 4MB app flash to a 2MB board).
*   **Evidence**: `esp32_device.rs` -> `verify_device()`, `flash_device()`.
