# Flash Error: Failed to Connect

## Error Pattern
```
Failed to connect to ESP32-S3
A fatal error occurred: Failed to connect
Serial port /dev/ttyUSB0 not found
```

## Issue Type
**flash_error**

## Component
**esptool**, **bootloader**

## Root Causes

1. **Wrong serial port**: Port not found or incorrect
2. **USB driver issue**: CP210x, CH340, or FTDI driver not installed
3. **Boot mode issue**: Board not in download mode
4. **Permission issue**: User doesn't have access to serial port
5. **Port in use**: Another program is using the serial port

## Solutions

### 1. Check Serial Port
```bash
# Linux/Mac
ls /dev/ttyUSB* /dev/ttyACM* /dev/cu.*

# Windows
# Check Device Manager for COM ports
```

### 2. Install USB Drivers
- **CP210x**: Download from Silicon Labs
- **CH340**: Download from manufacturer
- **FTDI**: Download from FTDI website

### 3. Fix Permissions (Linux)
```bash
sudo usermod -a -G dialout $USER
# Log out and back in, or:
newgrp dialout
```

### 4. Manual Boot Mode (ESP32-S3-DevKitC)
- Hold **BOOT** button (GPIO0)
- Press and release **EN** (Reset) button
- Release **BOOT** button
- Board is now in download mode

### 5. Specify Port Explicitly
```bash
idf.py -p /dev/ttyUSB0 flash
# or
esptool.py --chip esp32s3 --port /dev/ttyUSB0 flash_id
```

### 6. Lower Baud Rate
```bash
idf.py -b 115200 flash
```

### 7. Check Port Not in Use
```bash
# Linux
lsof /dev/ttyUSB0
# Kill process if needed
```

## ESP32-S3-DevKitC Specific Notes

- **Strapping pins**: GPIO0, GPIO3, GPIO45, GPIO46
- **Boot button**: GPIO0 (must be LOW for download mode)
- **USB**: Native USB on GPIO19/20 (USB Serial/JTAG)
- **Default UART**: UART0 on GPIO43/44

## Verification
```bash
# Test connection
esptool.py --chip esp32s3 --port /dev/ttyUSB0 chip_id
```

## Related
- Flashing Troubleshooting Guide
- Serial Connection Establishment
- esptool documentation
