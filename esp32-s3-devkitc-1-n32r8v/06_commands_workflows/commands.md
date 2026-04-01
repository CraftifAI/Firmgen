# ESP32-S3 Development Commands & Workflows

## Board: esp32-s3-devkitc
## Variant: 16N8R
## Target: esp32s3
## IDF Version: v6.1-dev-1163-g7e7773e0b8-dirty

## idf.py Commands

### Basic Workflow

```bash
# Set target (if not already set)
idf.py set-target esp32s3

# Configure project
idf.py menuconfig

# Build project
idf.py build

# Flash firmware
idf.py flash

# Monitor serial output
idf.py monitor

# Flash and monitor (combined)
idf.py flash monitor

# Clean build
idf.py fullclean

# Save configuration
idf.py save-defconfig
```

### Advanced Commands

```bash
# Show project size
idf.py size

# Show component dependencies
idf.py show_efuse_table

# Reconfigure CMake
idf.py reconfigure
```

## esptool.py Commands

### Basic Flashing

```bash
# Erase entire flash
esptool.py --chip esp32s3 --port /dev/ttyUSB0 erase_flash

# Write flash
esptool.py --chip esp32s3 --port /dev/ttyUSB0 write_flash 0x1000 bootloader.bin

# Read flash
esptool.py --chip esp32s3 --port /dev/ttyUSB0 read_flash 0x0 0x100000 flash_dump.bin
```

### Common Issues

#### Port Not Found
- Check USB connection
- Install USB drivers (CP210x, CH340, etc.)
- Check permissions: `sudo usermod -a -G dialout $USER` (Linux)

#### Flash Failed
- Hold BOOT button during flash
- Check baud rate (try lower: `--baud 115200`)
- Verify chip type: `esptool.py --chip esp32s3 chip_id`

#### Boot Loop
- Check partition table size
- Verify flash size matches board
- Check for brownout (insufficient power)

## Serial Monitor

### Basic Usage
```bash
idf.py monitor
```

### Filtering Logs
```bash
idf.py monitor --print-filter="wifi:* esp_netif:*"
```

### Exit Monitor
Press `Ctrl+]` to exit

## Troubleshooting

### Build Errors
- **Undefined reference**: Missing component dependency in CMakeLists.txt
- **Config not found**: Run `idf.py menuconfig` to set CONFIG_*
- **Version mismatch**: Check ESP-IDF version compatibility

### Flash Errors
- **Failed to connect**: Wrong port, driver issue, or boot mode
- **Timeout**: Lower baud rate or check USB cable
- **Verify failed**: Flash chip may be damaged

### Runtime Errors
- **Guru Meditation**: Stack overflow, null pointer, or invalid memory access
- **Brownout**: Insufficient power supply (check current rating)
- **WiFi not connecting**: Check SSID, password, signal strength

## Board-Specific Notes

### esp32-s3-devkitc (16N8R)

- **USB Port**: Native USB on ESP32-S3 (GPIO19/20)
- **Boot Mode**: Hold BOOT button (GPIO46) during reset
- **Default UART**: UART0 on GPIO43/44
- **Flash Size**: Check with `esptool.py flash_id`
- **Flash Size**: 16MB
- **PSRAM Size**: 8MB
- **Partition Table**: Use partition table compatible with 16MB flash

