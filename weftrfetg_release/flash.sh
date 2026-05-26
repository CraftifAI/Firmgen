#!/usr/bin/env bash
# Factory flash script for weftrfetg
# Usage: bash flash.sh [PORT]
#   PORT defaults to /dev/ttyUSB0 (override with first argument or FLASH_PORT env var)
set -euo pipefail

PORT="${FLASH_PORT:-${1:-/dev/ttyUSB0}}"

echo "[INFO] Flashing weftrfetg to $PORT ..."
esptool.py --chip esp32s3 --port "$PORT" --baud 460800 write_flash \
  --flash_mode dio --flash_size 2MB --flash_freq 80m \
  0x0 firmware/bootloader.bin \
  0x8000 firmware/partition-table.bin \
  0x10000 firmware/wifi_station.bin

echo "[OK] Flash complete."
