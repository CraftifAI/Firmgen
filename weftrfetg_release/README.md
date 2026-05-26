# weftrfetg — Factory Release

## Quick Start

1. Extract this ZIP
2. Connect your esp32s3 board via USB
3. Run the flash script:

### Windows
```
flash.bat COM3
```

### Linux / macOS
```bash
bash flash.sh /dev/ttyUSB0
```

Replace the port with your actual serial port.

## Manual Flash Command

If you prefer to run esptool directly:

```bash
esptool.py --chip esp32s3 --port PORT --baud 460800 write_flash \
  --flash_mode dio --flash_size 2MB --flash_freq 80m \
  0x0 firmware/bootloader.bin \
  0x8000 firmware/partition-table.bin \
  0x10000 firmware/wifi_station.bin
```

## Firmware Details

| Setting    | Value        |
|------------|--------------|
| Chip       | `esp32s3`     |
| Flash mode | `dio` |
| Flash size | `2MB` |
| Flash freq | `80m` |

## Prerequisites

- **esptool** must be installed and on your PATH
  - Install via pip: `pip install esptool`
  - Or download standalone: https://github.com/espressif/esptool/releases
- USB drivers for your board (CP210x, CH340, FTDI, etc.)

## Files

- `flash_config.json` — Machine-readable flash configuration
- `firmware/` — Binary firmware files
- `flash.bat` — Windows flash script
- `flash.sh` — Linux/macOS flash script
- `SHA256SUMS.txt` — File integrity checksums
