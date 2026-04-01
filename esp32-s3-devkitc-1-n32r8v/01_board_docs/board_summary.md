# ESP32-S3-DevKitC-1 Board Summary

## Board Information

- **Board Name**: ESP32-S3-DevKitC-1
- **Variant**: N32R8V (32MB Flash + 8MB PSRAM)
- **SoC**: ESP32-S3
- **Manufacturer**: Espressif Systems

## Key Specifications

### Power
- **Input**: USB-C (5V)
- **Typical Current**: 100-300mA (idle), 500mA+ (WiFi active)
- **Power Supply**: USB port or external 5V
- **Brownout Protection**: Enabled by default (configurable)

### Connectivity
- **USB**: Native USB Serial/JTAG on GPIO19/20
- **UART**: UART0 default on GPIO43 (TX), GPIO44 (RX)
- **WiFi**: 2.4 GHz only (802.11 b/g/n)
- **Bluetooth**: BLE 5.0

### Memory (N32R8V Variant)
- **Flash**: 32MB (SPI/QSPI)
- **PSRAM**: 8MB (SPI/QSPI)
- **Internal SRAM**: 512KB

### GPIO
- **Total GPIOs**: 45 physical pins (GPIO0-21, GPIO26-48)
- **Strapping Pins**: GPIO0, GPIO3, GPIO45, GPIO46
- **Flash/PSRAM Pins**: GPIO26-32 (Quad) or GPIO26-37 (Octal)
- **USB Pins**: GPIO19 (D-), GPIO20 (D+)
- **RTC GPIOs**: Available for low-power operation

## Boot and Reset

### Boot Mode
- **Normal Boot**: GPIO0 HIGH (default)
- **Download Mode**: GPIO0 LOW (hold BOOT button)
- **Manual Entry**: Hold BOOT button, press and release EN button

### Strapping Pins
- **GPIO0**: Boot mode selection
- **GPIO3**: Flash voltage selection
- **GPIO45**: Flash voltage selection
- **GPIO46**: Boot mode selection

## Physical Connectors

- **USB-C**: Power and programming/debugging
- **GPIO Headers**: Breakout for all GPIOs
- **Reset Button**: EN (chip enable/reset)
- **Boot Button**: GPIO0 (boot mode selection)

## Pinout Reference

### Critical Pins
- **GPIO0**: Boot button, strapping pin
- **GPIO19/20**: USB Serial/JTAG (native)
- **GPIO26-32**: SPI Flash/PSRAM (do not reconfigure)
- **GPIO43/44**: UART0 (default console)

### Peripheral Pins (Examples)
- **SPI**: GPIO11 (MOSI), GPIO13 (MISO), GPIO12 (CLK), GPIO10 (CS)
- **I2C**: GPIO8 (SDA), GPIO9 (SCL) - example
- **ADC**: GPIO1-10 (ADC1), GPIO11-20 (ADC2, conflicts with WiFi)
- **PWM/LEDC**: GPIO38 (on-board RGB LED for v1.1; v1.0 uses GPIO48), GPIO1-21 (alternate pins)
- **Timer**: Available on all GPIOs via LEDC

## Development Workflow

1. **Connect**: USB-C cable to computer
2. **Set Target**: `idf.py set-target esp32s3`
3. **Configure**: `idf.py menuconfig` (or use board preset)
4. **Build**: `idf.py build`
5. **Flash**: `idf.py flash` (auto-enters boot mode)
6. **Monitor**: `idf.py monitor`

## Troubleshooting

- **Can't flash**: Hold BOOT, press EN, release BOOT
- **Port not found**: Check USB drivers (CP210x/CH340)
- **Brownout**: Use quality USB cable, external power if needed
- **WiFi not working**: Verify 2.4 GHz band, check signal strength

## Documentation Sources

- **Official Docs**: https://docs.espressif.com/projects/esp-dev-kits/
- **Hardware Reference**: `$IDF_PATH/docs/en/hw-reference/esp32s3/`
- **Datasheet**: https://www.espressif.com/en/support/download/documents
- **Schematic**: Available from Espressif downloads

## Related
- ESP32-S3 Technical Reference Manual
- Hardware Design Guidelines
- Getting Started Guide
