# Runtime Error: Guru Meditation - IllegalInstruction

## Error Pattern
```
Guru Meditation Error: Core 0 panic'ed (IllegalInstruction). Exception was unhandled.
```

## Issue Type
**runtime_error**, **panic**

## Component
**esp_system**, **panic_handler**

## Root Causes

1. **FreeRTOS task function returned**: Task function must not return; use `vTaskDelete(NULL)` instead
2. **SPI flash pins reconfigured**: Flash pins used for other functions (GPIO, UART, etc.)
3. **External device interference**: Device connected to SPI flash pins
4. **C++ missing return**: Non-void function exits without return value (with optimizations)

## Solutions

### 1. FreeRTOS Task Function Returned
```c
// BAD - Task function returns
void my_task(void *pvParameters) {
    // do work
    return;  // CRASH! Task must not return
}

// GOOD - Task deletes itself
void my_task(void *pvParameters) {
    // do work
    vTaskDelete(NULL);  // Correct way to terminate
}
```

### 2. SPI Flash Pins Reconfigured
**Problem**: GPIO26-32 (and GPIO33-37 for Octal) are used for SPI flash/PSRAM. Cannot be reconfigured.

**Solution**:
- Never reconfigure GPIO26-32 as regular GPIO
- Never reconfigure GPIO33-37 on boards with Octal flash/PSRAM
- Check Hardware Design Guidelines for pin constraints
- Use other GPIOs for your peripherals

### 3. External Device on Flash Pins
**Problem**: External device accidentally connected to SPI flash pins interferes with communication.

**Solution**:
- Check board schematic
- Ensure no external connections to GPIO26-32
- Verify board layout matches design

### 4. C++ Missing Return (Optimized Build)
```cpp
// BAD - Missing return in optimized build
int my_function() {
    if (condition) {
        return 1;
    }
    // Missing return - compiler may omit epilogue
}

// GOOD
int my_function() {
    if (condition) {
        return 1;
    }
    return 0;  // Always return a value
}
```

**Prevention**: ESP-IDF enables `-Werror=return-type` by default. Don't disable compiler warnings.

## ESP32-S3-DevKitC Specific

- **SPI Flash pins**: GPIO26-32 (Quad) or GPIO26-37 (Octal)
- **PSRAM pins**: Same as flash pins
- **Do NOT use**: GPIO26-32 for any other purpose
- **Safe GPIOs**: GPIO0-21, GPIO33-48 (except flash/PSRAM pins)

## Debugging

1. **Check backtrace**: Identify which function caused the illegal instruction
2. **Check task functions**: Ensure no task functions return
3. **Verify GPIO configuration**: Check if flash pins were reconfigured
4. **Review recent changes**: What GPIO/peripheral changes were made?

## Prevention

- Always use `vTaskDelete(NULL)` in FreeRTOS tasks
- Never reconfigure SPI flash pins
- Always return values from non-void functions
- Keep compiler warnings enabled
- Follow Hardware Design Guidelines

## Related
- Fatal Errors documentation
- FreeRTOS Task Management
- Hardware Design Guidelines
- GPIO Pin Constraints
