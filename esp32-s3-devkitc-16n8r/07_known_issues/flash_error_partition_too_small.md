# Flash Error: Partition Too Small

## Error Pattern
```
Error: Partition "factory" is too small (app is X bytes, partition is Y bytes)
Error: Image size X is larger than partition size Y
```

## Issue Type
**flash_error**, **build_error**

## Component
**partition_table**, **bootloader**

## Root Cause
Application binary is larger than the partition allocated for it in the partition table.

## Common Causes

1. **Partition size too small**: Factory/OTA partition size insufficient
2. **Application too large**: Code size increased (new features, debug symbols)
3. **Wrong partition table**: Using wrong partition scheme for flash size
4. **Flash size mismatch**: CONFIG_ESPTOOLPY_FLASHSIZE doesn't match hardware

## Solutions

### 1. Increase Partition Size
```csv
# In partitions.csv or menuconfig
# Name,   Type, SubType, Offset,  Size, Flags
factory,  app,  factory, 0x10000, 2M,   # Increase from 1M to 2M
```

Or in menuconfig:
- Component config → Partition Table → Factory app size

### 2. Reduce Application Size
```c
// Enable size optimizations
CONFIG_COMPILER_OPTIMIZATION_SIZE=y

// Disable debug features
CONFIG_LOG_DEFAULT_LEVEL_WARN=y  // Reduce logging
CONFIG_ESP_ERR_TO_NAME_LOOKUP=n  // Disable error name lookup

// Remove unused components
// Check CMakeLists.txt for unnecessary REQUIRES
```

### 3. Check Flash Size Configuration
```c
// In menuconfig: Serial flasher config → Flash size
CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y  // For 16N8R variant
```

### 4. Use Appropriate Partition Table
For 16MB flash (16N8R variant):
- Use "Factory app, two OTA definitions" (recommended)
- Or custom partition table with larger app partitions

### 5. Enable Partition Table Resize
```c
// Allow automatic partition sizing
CONFIG_PARTITION_TABLE_CUSTOM=y
CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"
```

## ESP32-S3-DevKitC 16N8R Specific

- **Flash size**: 16MB
- **Recommended partition scheme**: OTA with 2-3MB app partitions
- **PSRAM**: 8MB (doesn't affect flash partitions)

## Example Partition Table (16MB Flash)

```csv
# Name,   Type, SubType, Offset,  Size,    Flags
nvs,      data, nvs,     0x9000,  0x6000,
otadata,  data, ota,     0xd000,  0x2000,
phy_init, data, phy,     0xf000,  0x1000,
factory,  app,  factory, 0x10000, 2M,      # 2MB for factory app
ota_0,    app,  ota_0,   ,        2M,      # 2MB for OTA_0
ota_1,    app,  ota_1,   ,        2M,      # 2MB for OTA_1
spiffs,   data, spiffs,  ,        4M,      # 4MB for SPIFFS
```

## Debugging

1. **Check app size**: `idf.py size` or `idf.py size-components`
2. **Check partition table**: `idf.py partition-table`
3. **Verify flash size**: `esptool.py --chip esp32s3 flash_id`
4. **Review recent changes**: What increased code size?

## Prevention

- Monitor application size during development
- Use appropriate partition table for flash size
- Enable size optimizations in release builds
- Plan partition sizes based on expected app size
- Leave headroom for future growth

## Related
- Partition Tables documentation
- Application Size Analysis
- Flash Configuration
