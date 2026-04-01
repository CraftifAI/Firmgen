# ESP32-S3 SoC Reference Summary

## SoC Overview

- **Name**: ESP32-S3
- **Architecture**: Dual-core Xtensa LX7 (32-bit)
- **CPU Frequency**: 80-240 MHz (configurable)
- **Process**: 40nm

## Core Architecture

### CPU
- **Cores**: 2 x Xtensa LX7
- **Frequency**: 80, 160, 240 MHz (default: 240 MHz)
- **Cache**: 
  - Instruction cache: 32KB (configurable)
  - Data cache: 32KB (configurable)
- **FPU**: Single-precision floating-point unit

### Memory Architecture

#### Internal Memory
- **SRAM**: 512KB
- **ROM**: Boot ROM with initial bootloader

#### External Memory
- **Flash**: Up to 16MB (SPI/QSPI/OPI)
- **PSRAM**: Up to 8MB (SPI/QSPI/OPI)
- **Memory Mapping**: Flash and PSRAM memory-mapped

### Memory Regions
- **IRAM**: Instruction RAM (0x40080000+)
- **DRAM**: Data RAM (0x3FC00000+)
- **Flash**: Memory-mapped (0x42000000+)
- **PSRAM**: Memory-mapped (0x3D000000+)

## Peripherals

### Communication Interfaces
- **UART**: 3 x UART controllers
- **SPI**: 3 x SPI controllers (master/slave)
- **I2C**: 2 x I2C controllers
- **I2S**: 2 x I2S controllers
- **USB**: USB Serial/JTAG, USB OTG
- **TWAI**: 1 x TWAI controller (CAN)

### Analog
- **ADC**: 2 x 12-bit SAR ADC (20 channels total)
- **DAC**: 2 x 8-bit DAC channels

### Digital
- **GPIO**: 45 physical GPIO pins
- **GPIO Matrix**: Flexible pin routing
- **RTC GPIO**: Available in sleep modes
- **Touch**: 14 x capacitive touch sensors

### Timers and PWM
- **Timer**: 4 x 64-bit general-purpose timers
- **LEDC**: 8 x LED PWM channels
- **MCPWM**: 2 x motor control PWM units
- **RMT**: 8 x remote control channels

### Other Peripherals
- **RTC**: Real-time clock with calendar
- **Temperature Sensor**: Internal temperature sensor
- **Watchdog**: Interrupt watchdog, task watchdog, RTC watchdog
- **DMA**: Direct Memory Access controllers

## Wireless

### WiFi
- **Standard**: 802.11 b/g/n
- **Band**: 2.4 GHz only
- **Modes**: Station, AP, Station+AP
- **Features**: WPA/WPA2/WPA3, WPS, 802.11n

### Bluetooth
- **Standard**: Bluetooth 5.0
- **Modes**: BLE (Bluetooth Low Energy)
- **Features**: BLE 5.0, mesh support

## Power Management

### Power Domains
- **VDD3P3**: Digital power (3.3V)
- **VDD3P3_RTC**: RTC power domain
- **VDD_SDIO**: SDIO power (1.8V/3.3V)

### Sleep Modes
- **Light Sleep**: CPU stopped, peripherals on
- **Deep Sleep**: RTC only, very low power
- **Hibernation**: Lowest power, RTC slow clock only

### Power Consumption (Typical)
- **Active (WiFi TX)**: ~240mA @ 3.3V
- **Active (WiFi RX)**: ~100mA @ 3.3V
- **Light Sleep**: ~0.8mA
- **Deep Sleep**: ~10µA (RTC only)

## Boot and Strapping

### Boot Modes
- **Normal Boot**: Boot from flash
- **Download Mode**: UART download for flashing
- **Strapping Pins**: GPIO0, GPIO3, GPIO45, GPIO46

### Boot Sequence
1. ROM bootloader (hardcoded)
2. Second-stage bootloader (from flash)
3. Application (from flash)

## Security Features

- **Secure Boot**: Hardware-based secure boot (v2)
- **Flash Encryption**: AES-256 flash encryption
- **eFuse**: One-time programmable fuses
- **Digital Signature**: RSA-based signature verification

## Clock System

### Clock Sources
- **External Crystal**: 40 MHz (typical)
- **Internal RC**: 8 MHz, 150 kHz
- **RTC**: 32.768 kHz (external or internal)

### Clock Tree
- **CPU Clock**: 80-240 MHz (from PLL)
- **APB Clock**: 80 MHz (from CPU)
- **RTC Clock**: 32.768 kHz

## Technical Reference

- **TRM**: ESP32-S3 Technical Reference Manual
- **Datasheet**: ESP32-S3 Datasheet
- **Errata**: ESP32-S3 Chip Errata
- **Hardware Design Guidelines**: PCB layout and design rules

## Related
- ESP32-S3 Technical Reference Manual (PDF)
- Hardware Design Guidelines
- Peripheral API Documentation
