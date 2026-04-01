# Runtime Error: Brownout Detector Triggered

## Error Pattern
```
Brownout detector was triggered
```

## Issue Type
**runtime_error**, **power**

## Component
**esp_system**, **brownout_detector**

## Root Cause
Supply voltage dropped below safe operating level. The brownout detector (BOD) is enabled by default and resets the chip when voltage is too low.

## Common Causes

1. **Insufficient power supply**: USB port doesn't provide enough current
2. **Poor USB cable**: High resistance cable causes voltage drop
3. **High current draw**: Application draws more current than supply can provide
4. **Power supply instability**: Unstable or noisy power supply
5. **Brownout level too sensitive**: BOD threshold set too high

## Solutions

### 1. Check Power Supply
- **USB 2.0**: Provides up to 500mA (may not be enough)
- **USB 3.0**: Provides up to 900mA
- **USB-C PD**: Can provide more current
- **External power**: Use external 5V supply with sufficient current rating

### 2. Use Quality USB Cable
- Use short, high-quality USB cable
- Avoid long or thin cables (high resistance)
- Use USB-C cable if board supports it

### 3. Reduce Current Draw
- Disable unnecessary peripherals
- Reduce CPU frequency: `CONFIG_ESP32S3_DEFAULT_CPU_FREQ_MHZ`
- Use light sleep when idle
- Disable PSRAM if not needed
- Reduce WiFi transmit power

### 4. Adjust Brownout Level
```c
// In menuconfig:
// Component config → ESP32-S3-Specific → Brownout detector voltage
// Options: 2.43V, 2.51V, 2.59V, 2.67V, 2.75V, 2.84V, 2.92V, 3.10V
CONFIG_ESP_BROWNOUT_DET_LVL_SEL_7  // 3.10V (less sensitive)
```

**Note**: Lowering brownout level may allow operation at unsafe voltages. Use with caution.

### 5. Disable Brownout (Not Recommended)
```c
// Only for development/debugging
CONFIG_ESP_BROWNOUT_DET=n
```

**Warning**: Disabling brownout can cause data corruption and flash wear. Not recommended for production.

## ESP32-S3-DevKitC Specific

- **Power input**: USB-C connector
- **Typical current**: 100-300mA (idle), 500mA+ (WiFi active)
- **16N8R variant**: 16MB flash + 8MB PSRAM may draw more current
- **USB Serial/JTAG**: Uses GPIO19/20, doesn't require external UART bridge

## Measurement

Monitor voltage during operation:
```bash
# Use multimeter to measure VDD at test point
# Should stay above 3.0V under load
```

## Debugging Steps

1. **Check voltage under load**: Measure with multimeter during operation
2. **Monitor current draw**: Use USB power meter
3. **Check for voltage drops**: Measure at different points on board
4. **Review power consumption**: Check component current requirements
5. **Test with external supply**: Use 5V external supply to isolate USB issue

## Prevention

- Use quality USB cable and power supply
- Design for power efficiency
- Monitor power consumption during development
- Test with realistic load conditions
- Keep brownout detector enabled in production

## Related
- Power Management documentation
- Current Consumption Measurement
- Hardware Design Guidelines
- Brownout Detector Configuration
