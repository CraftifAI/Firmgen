# Build Error: CONFIG Symbol Not Found

## Error Pattern
```
error: 'CONFIG_XXX' undeclared (first use in this function)
CMake Error: CONFIG_XXX is not set
```

## Issue Type
**build_error**, **kconfig**

## Component
**build_system**, **kconfig**

## Root Cause
Configuration symbol is not defined because:
1. Symbol doesn't exist in Kconfig
2. Symbol is not enabled (depends on other configs)
3. `menuconfig` was not run after adding new configs
4. Symbol is target-specific and not available for ESP32-S3

## Solutions

### 1. Check if Symbol Exists
```bash
# Search for config in Kconfig files
grep -r "config CONFIG_XXX" $IDF_PATH/components/
```

### 2. Run menuconfig
```bash
idf.py menuconfig
# Navigate to the config option
# Enable it if needed
# Save and exit
```

### 3. Check Dependencies
Some configs depend on other configs:
```bash
# In menuconfig, check "Help" for the config
# It will show dependencies like:
# "depends on: CONFIG_XXX && CONFIG_YYY"
```

### 4. Check Target Compatibility
Some configs are target-specific:
- `CONFIG_ESP32_*` - ESP32 only
- `CONFIG_ESP32S3_*` - ESP32-S3 only
- `CONFIG_ESP32C3_*` - ESP32-C3 only

For ESP32-S3, use `CONFIG_ESP32S3_*` configs.

### 5. Set Config in sdkconfig.defaults
```ini
# Create or edit sdkconfig.defaults
CONFIG_XXX=y
CONFIG_YYY=value
```

Then run:
```bash
idf.py reconfigure
```

### 6. Check Component Requirements
Some configs are only available when component is included:
```cmake
# In CMakeLists.txt
idf_component_register(
    REQUIRES esp_wifi  # Component must be included
)
```

## Common ESP32-S3 Config Symbols

- `CONFIG_ESP32S3_DEFAULT_CPU_FREQ_MHZ` - CPU frequency
- `CONFIG_ESP32S3_DATA_CACHE_SIZE` - Data cache size
- `CONFIG_ESP32S3_INSTRUCTION_CACHE_SIZE` - Instruction cache size
- `CONFIG_ESPTOOLPY_FLASHSIZE` - Flash size
- `CONFIG_SPIRAM` - PSRAM support

## Debugging

1. **List all configs**: `idf.py show_efuse_table`
2. **Check sdkconfig**: `cat sdkconfig | grep CONFIG_XXX`
3. **Verify target**: `idf.py set-target esp32s3`
4. **Reconfigure**: `idf.py reconfigure`

## Prevention

- Always run `menuconfig` after adding new features
- Check component dependencies before using configs
- Use `sdkconfig.defaults` for project defaults
- Verify target is set correctly: `idf.py set-target esp32s3`

## Related
- Kconfig System documentation
- Build System guide
- ESP32-S3 Specific Configuration
