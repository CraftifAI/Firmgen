---
title: "ESP32 Tools (ESP-IDF)"
---

## Overview

Refact now ships a native ESP32 toolset (ESP-IDF). Tools are loaded selectively via `--platform esp32` (or combined with `c2000` as `--platform c2000,esp32`). Configuration is read from `~/.cache/refact/esp32_tools.yaml` or `/v1/esp32-config`.

### Tools
- `esp32_project` — create/list/validate projects
- `esp32_build` — build/clean/reconfigure and size analysis
- `esp32_device` — detect/flash/monitor/erase/info
- `esp32_config` — sdkconfig/partition/wifi/gpio (basic)
- `esp32_component` — add/list/search components
- `esp32_analyze` — code analysis with ESP-IDF specific checks

## What’s Implemented (Current State)

- Project creation: `idf.py create-project-from-example` (examples under `$IDF_PATH/examples`) with target set via `idf.py set-target`.
- Example listing: recursive scan of `$IDF_PATH/examples`, filter support, grouped by category.
- Build: `idf.py build`, `fullclean`, `reconfigure`; reports binary path/size; runs `idf.py size` to report DRAM/IRAM/Flash usage.
- Device:
  - Detect: esptool auto-detect (no port) plus `/dev/ttyUSB*`/`ttyACM*` scan.
  - Flash: `idf.py -p <port> -b <baud> flash` from project directory (requires prior build).
  - Monitor: `idf.py monitor --no-reset` with timeout, filters monitor control lines.
  - Erase/info: `idf.py erase-flash`, `esptool chip_id`.
- Components:
  - Add: `idf.py add-dependency <component>`.
  - List: parses `idf_component.yml` (root and `main/`) and `managed_components/`.
  - Search: queries ESP Component Registry API.
- Analyze: ESP-IDF-focused static checks (memory, FreeRTOS delays/stack, Wi‑Fi init, GPIO direction, error-checking density, deprecated APIs, ISR safety).

## Key Behaviors & Flags

- Platform gating: tools load only when `--platform` includes `esp32`.
- Baud rate support: `esp32_device` accepts `baud_rate` for flash/monitor.
- Project scoping: flash/monitor run from project dir for ELF decoding.
- Size reporting: build reports `.bin`/`.elf` paths and `idf.py size` summary.

## Configuration

Edit `~/.cache/refact/esp32_tools.yaml`:
```yaml
esp32_config:
  esp_idf_path: "/home/user/esp/esp-idf"
  projects_path: "/home/user/esp/projects"
  default_target: "esp32p4"
  default_serial_port: "/dev/ttyUSB0"
  default_baud_rate: 115200
tools:
  esp32_project: { enabled: true }
  esp32_build: { enabled: true }
  esp32_device: { enabled: true }
  esp32_config: { enabled: true }
  esp32_component: { enabled: true }
  esp32_analyze: { enabled: true }
```

## Gaps / Next Steps
- Partition and GPIO configuration: implement real handlers (currently stubs).
- Component removal/search robustness; better parsing of registry results.
- Add `idf.py size-components` and optional `idf.py flash monitor` combo op.
- Smarter device detection (Windows/macOS paths); optional auto-reset/boot pins.
- OTA, flash encryption/secure boot, JTAG/OpenOCD workflows.

## More Information

For detailed setup instructions, troubleshooting, and contribution guidelines, see the internal reference doc: `p_docs/ESP32_TOOLS_REFERENCE.md`

