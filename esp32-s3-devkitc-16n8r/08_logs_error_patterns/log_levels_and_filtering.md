# Log Levels and Filtering

## Log Levels

ESP-IDF uses the following log levels (from lowest to highest verbosity):

- **NONE** (0): No logs
- **ERROR** (1): Critical errors that may cause system failure
- **WARN** (2): Warning messages for potentially harmful situations
- **INFO** (3): Informational messages about normal operation
- **DEBUG** (4): Detailed debug messages for development
- **VERBOSE** (5): Very detailed debug messages (most verbose)

## Setting Log Levels

### Component-Level Logging
```c
#include "esp_log.h"

// Set log level for specific component
esp_log_level_set("wifi", ESP_LOG_DEBUG);
esp_log_level_set("esp_netif", ESP_LOG_INFO);
esp_log_level_set("*", ESP_LOG_WARN);  // Default for all components
```

### Build-Time Configuration
```ini
# In sdkconfig or menuconfig
CONFIG_LOG_DEFAULT_LEVEL_DEBUG=y
CONFIG_LOG_DEFAULT_LEVEL=4  # DEBUG

# Component-specific (if supported)
CONFIG_LOG_MAXIMUM_LEVEL=5  # VERBOSE
```

## Log Filtering in Monitor

### Filter by Component
```bash
idf.py monitor --print-filter="wifi:* esp_netif:*"
```

### Filter by Level
```bash
# Show only ERROR and WARN
idf.py monitor --print-filter="*:E *:W"

# Show all levels
idf.py monitor --print-filter="*:V"
```

### Combined Filtering
```bash
# WiFi component at DEBUG level, all others at INFO
idf.py monitor --print-filter="wifi:D *:I"
```

## Common Log Patterns

### Error Logs
```
E (12345) component: Error message
```
- **E**: Error level
- **12345**: Timestamp (milliseconds since boot)
- **component**: Component name
- **Error message**: Error description

### Warning Logs
```
W (12345) component: Warning message
```

### Info Logs
```
I (12345) component: Info message
```

### Debug Logs
```
D (12345) component: Debug message
```

## Logging Best Practices

1. **Use appropriate levels**: ERROR for failures, INFO for normal flow, DEBUG for development
2. **Include context**: Add relevant values (error codes, states, etc.)
3. **Avoid excessive logging**: Too many logs impact performance
4. **Use tags**: Use component/module names as tags
5. **Conditional compilation**: Use `#ifdef CONFIG_LOG_LEVEL_DEBUG` for expensive debug logs

## Example Usage

```c
#include "esp_log.h"

static const char *TAG = "my_component";

void my_function(void) {
    ESP_LOGI(TAG, "Starting operation");
    
    esp_err_t ret = some_operation();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Operation failed: %s", esp_err_to_name(ret));
        return;
    }
    
    ESP_LOGD(TAG, "Operation completed successfully, value: %d", value);
}
```

## Backtrace Decoding

When using `idf.py monitor`, backtraces are automatically decoded:
```
Backtrace: 0x400e14ed:0x3ffb5030 0x400d0802:0x3ffb5050
0x400e14ed: app_main at /path/to/main.c:36
0x400d0802: main_task at /path/to/cpu_start.c:470
```

Without monitor, use `addr2line`:
```bash
xtensa-esp32s3-elf-addr2line -pfiaC -e build/app.elf 0x400e14ed
```

## Related
- Logging API documentation
- IDF Monitor documentation
- Error Handling guide
