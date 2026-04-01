# ESP32-S3 Peripherals Summary

## SoC: ESP32-S3
## Architecture: Dual-core Xtensa LX7

## Communication Interfaces

### UART
- **Controllers**: 3 x UART
- **Features**: Hardware flow control, RS485 mode, IRDA mode
- **Default**: UART0 on GPIO43/44 (console)
- **API**: `driver/uart.h`

### SPI
- **Controllers**: 3 x SPI (master/slave)
- **Features**: Full-duplex, half-duplex, DMA support
- **Speed**: Up to 80 MHz (master), 40 MHz (slave)
- **API**: `driver/spi_master.h`, `driver/spi_slave.h`
- **Note**: SPI0/1 used for flash/PSRAM, use SPI2/3 for application

### I2C
- **Controllers**: 2 x I2C
- **Features**: Master/slave mode, 7-bit/10-bit addressing
- **Speed**: Standard (100kHz), Fast (400kHz), Fast Plus (1MHz)
- **API**: `driver/i2c_master.h`, `driver/i2c_slave.h`

### I2S
- **Controllers**: 2 x I2S
- **Features**: Audio, LCD parallel mode, DMA support
- **API**: `driver/i2s.h`

### USB
- **USB Serial/JTAG**: Native USB on GPIO19/20
- **USB OTG**: USB On-The-Go support
- **API**: `driver/usb_serial_jtag.h`, `driver/usb_host.h`, `driver/usb_device.h`

### TWAI (CAN)
- **Controllers**: 1 x TWAI
- **Features**: CAN 2.0A/B protocol
- **API**: `driver/twai.h`

## Analog Interfaces

### ADC
- **Controllers**: 2 x 12-bit SAR ADC
- **Channels**: 20 total (10 per ADC)
- **ADC1**: GPIO1-10
- **ADC2**: GPIO11-20 (conflicts with WiFi)
- **API**: `driver/adc/adc_oneshot.h`, `driver/adc/adc_continuous.h`

### DAC
- **Channels**: 2 x 8-bit DAC
- **GPIOs**: GPIO17, GPIO18
- **API**: `driver/dac.h`

## Digital Interfaces

### GPIO
- **Total**: 45 physical GPIO pins
- **GPIO Matrix**: Flexible pin routing
- **Features**: Input/output, pull-up/pull-down, interrupts, RTC GPIO
- **API**: `driver/gpio.h`

### Touch Sensor
- **Channels**: 14 x capacitive touch
- **API**: `driver/touch_sensor.h`

## Timers and PWM

### General Purpose Timer
- **Timers**: 4 x 64-bit timers
- **API**: `driver/gptimer.h`

### LEDC (LED PWM)
- **Channels**: 8 x LED PWM
- **Frequency**: Configurable
- **API**: `driver/ledc.h`

### MCPWM (Motor Control PWM)
- **Units**: 2 x MCPWM units
- **Features**: Motor control, synchronized operations
- **API**: `driver/mcpwm.h`

### RMT (Remote Control)
- **Channels**: 8 x RMT channels
- **Features**: IR remote, custom protocols
- **API**: `driver/rmt.h`

## Other Peripherals

### RTC
- **Features**: Real-time clock, calendar, alarms
- **API**: `driver/rtc.h`

### Temperature Sensor
- **Internal**: On-chip temperature sensor
- **API**: `driver/temp_sensor.h`

### Watchdog
- **Types**: Interrupt watchdog, Task watchdog, RTC watchdog
- **API**: `esp_task_wdt.h`, system watchdog APIs

### DMA
- **Controllers**: GDMA (General DMA)
- **Features**: Memory-to-memory, peripheral-to-memory transfers
- **API**: `driver/gdma.h`

## Wireless

### WiFi
- **Standard**: 802.11 b/g/n
- **Band**: 2.4 GHz only
- **Modes**: Station, AP, Station+AP
- **API**: `esp_wifi.h`

### Bluetooth
- **Standard**: Bluetooth 5.0
- **Modes**: BLE (Bluetooth Low Energy)
- **API**: `esp_bt.h`, `esp_ble_*` APIs

## Memory

### Internal SRAM
- **Size**: 512KB
- **Usage**: Code, data, heap

### External Flash
- **Interface**: SPI/QSPI/OPI
- **Size**: Up to 16MB (board dependent)
- **Memory-mapped**: Yes

### External PSRAM
- **Interface**: SPI/QSPI/OPI
- **Size**: Up to 8MB (board dependent)
- **Memory-mapped**: Yes

## Clock System

- **CPU Clock**: 80, 160, 240 MHz (configurable)
- **APB Clock**: 80 MHz
- **RTC Clock**: 32.768 kHz (external or internal)

## Power Management

- **Sleep Modes**: Light sleep, Deep sleep, Hibernation
- **Power Domains**: VDD3P3, VDD3P3_RTC, VDD_SDIO
- **API**: `esp_pm.h`, `esp_sleep.h`

## Related
- ESP32-S3 Technical Reference Manual
- Peripheral API Documentation
- Hardware Design Guidelines
