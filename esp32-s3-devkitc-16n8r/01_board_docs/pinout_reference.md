# ESP32-S3-DevKitC-1 Pinout Reference

## Board: ESP32-S3-DevKitC-1
## Variant: 16N8R (16MB Flash + 8MB PSRAM)
## SoC: ESP32-S3

## Critical Pins (Do Not Reconfigure)

### SPI Flash/PSRAM Pins
- **GPIO26-32**: SPI Flash and PSRAM (Quad mode)
- **GPIO33-37**: Additional pins for Octal Flash/PSRAM (if used)
- **⚠️ WARNING**: These pins MUST NOT be reconfigured as GPIO or other peripherals

### Strapping Pins
- **GPIO0**: Boot mode selection (LOW = download mode)
- **GPIO3**: Flash voltage selection
- **GPIO45**: Flash voltage selection  
- **GPIO46**: Boot mode selection

**Note**: Strapping pins are sampled at reset. Do not use for other functions.

### USB Serial/JTAG
- **GPIO19**: USB D- (Data Minus)
- **GPIO20**: USB D+ (Data Plus)
- **Default**: USB Serial/JTAG functionality
- **⚠️ WARNING**: If reconfigured, USB Serial/JTAG will be disabled

## Default Peripheral Assignments

### UART0 (Default Console)
- **GPIO43**: TX (Transmit)
- **GPIO44**: RX (Receive)
- **Usage**: Default serial console for logging

### USB Serial/JTAG (Native)
- **GPIO19**: D-
- **GPIO20**: D+
- **Usage**: Native USB for programming and debugging (no external bridge needed)

## GPIO Pin Summary

### Available GPIOs (Safe to Use)
- **GPIO0-21**: Available (except GPIO0, GPIO3 are strapping pins)
- **GPIO33-48**: Available (except GPIO45, GPIO46 are strapping pins, and GPIO33-37 if Octal flash/PSRAM)

### Restricted GPIOs
- **GPIO26-32**: SPI Flash/PSRAM (do not use)
- **GPIO33-37**: Octal Flash/PSRAM (if board has Octal, do not use)
- **GPIO0, GPIO3, GPIO45, GPIO46**: Strapping pins (use with caution)
- **GPIO19, GPIO20**: USB Serial/JTAG (reconfiguring disables USB)

## Pin Function Matrix

ESP32-S3 has a flexible GPIO matrix that allows routing most peripheral signals to any GPIO pin. However, some constraints apply:

- **SPI Flash/PSRAM pins**: Cannot be used for other functions
- **Strapping pins**: Sampled at boot, can be used after boot
- **USB pins**: Reconfiguring disables USB functionality

## Example Pin Assignments

### SPI (Example)
- **MOSI**: GPIO11
- **MISO**: GPIO13
- **CLK**: GPIO12
- **CS**: GPIO10

### I2C (Example)
- **SDA**: GPIO8
- **SCL**: GPIO9

### ADC (Example)
- **ADC1**: GPIO1-10
- **ADC2**: GPIO11-20 (conflicts with WiFi when active)

## Boot Mode Control

### Normal Boot
- GPIO0: HIGH (default)
- GPIO46: HIGH (default)

### Download Mode (Manual)
1. Hold **BOOT** button (GPIO0 LOW)
2. Press and release **EN** (Reset) button
3. Release **BOOT** button
4. Board is now in download mode

### Auto-Reset (Automatic)
- DTR/RTS from USB-to-UART bridge automatically control boot mode
- No manual button press needed for flashing

## Power and Reset

- **EN Pin**: Chip enable/reset (active LOW)
- **BOOT Pin**: GPIO0, boot mode selection
- **USB-C**: Power input (5V) and programming interface

## 16N8R Variant Specifics

- **Flash Size**: 16MB (SPI/QSPI)
- **PSRAM Size**: 8MB (SPI/QSPI)
- **Partition Table**: Use appropriate scheme for 16MB flash
- **Recommended**: OTA partition scheme with 2-3MB app partitions

## Related
- ESP32-S3 GPIO API documentation
- Hardware Design Guidelines
- Pin Function Matrix documentation
