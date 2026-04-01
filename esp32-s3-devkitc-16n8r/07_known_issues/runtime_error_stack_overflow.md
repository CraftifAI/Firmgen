# Runtime Error: Stack Overflow

## Error Pattern
```
Guru Meditation Error: Core 0 panic'ed (Stack protection fault).
Guru Meditation Error: Core 0 panic'ed (Breakpoint).  # RISC-V
Debug exception reason: Stack canary watchpoint triggered (task_name)
```

## Issue Type
**runtime_error**, **panic**

## Component
**freertos**, **esp_system**

## Root Cause
Task stack size is too small for the operations being performed, or infinite recursion occurred.

## Common Causes

1. **Stack size too small**: Task created with insufficient stack
2. **Large local variables**: Large arrays or structures on stack
3. **Deep function calls**: Deep call stack consumes stack
4. **Recursion**: Infinite or very deep recursion
5. **Interrupt stack overflow**: ISR stack too small

## Solutions

### 1. Increase Task Stack Size
```c
// Default main task stack
// In menuconfig: CONFIG_ESP_MAIN_TASK_STACK_SIZE

// For custom tasks
xTaskCreate(
    my_task,
    "my_task",
    4096,  // Increase from default (e.g., 2048 → 4096)
    NULL,
    5,
    NULL
);
```

### 2. Move Large Variables to Heap
```c
// BAD - Large array on stack
void my_function(void) {
    uint8_t large_buffer[8192];  // 8KB on stack!
    // ...
}

// GOOD - Allocate on heap
void my_function(void) {
    uint8_t *large_buffer = malloc(8192);
    if (large_buffer == NULL) {
        ESP_LOGE(TAG, "Failed to allocate buffer");
        return;
    }
    // ...
    free(large_buffer);
}
```

### 3. Check for Recursion
```c
// BAD - Infinite recursion
void recursive_function(void) {
    recursive_function();  // No base case!
}

// GOOD - Base case
void recursive_function(int depth) {
    if (depth <= 0) return;  // Base case
    recursive_function(depth - 1);
}
```

### 4. Enable Stack Monitoring
```c
// In menuconfig
CONFIG_FREERTOS_WATCHPOINT_END_OF_STACK=y
CONFIG_ESP_SYSTEM_HW_STACK_GUARD=y  // ESP32-S3 supports this
```

### 5. Check Stack Usage
```c
// Enable runtime stats
CONFIG_FREERTOS_GENERATE_RUN_TIME_STATS=y
CONFIG_FREERTOS_USE_TRACE_FACILITY=y

// Check stack high water mark
UBaseType_t stack_remaining = uxTaskGetStackHighWaterMark(NULL);
ESP_LOGI(TAG, "Stack remaining: %d bytes", stack_remaining * sizeof(StackType_t));
```

## ESP32-S3-DevKitC Specific

- **Main task stack**: Default 3584 bytes (configurable)
- **Interrupt stack**: Default 1536 bytes
- **Hardware stack guard**: Supported (CONFIG_ESP_SYSTEM_HW_STACK_GUARD)

## Debugging

1. **Check backtrace**: Identify which function caused overflow
2. **Enable stack guard**: CONFIG_ESP_SYSTEM_HW_STACK_GUARD
3. **Monitor stack usage**: Use uxTaskGetStackHighWaterMark()
4. **Review recent changes**: What code was added before crash?

## Prevention

- Always check stack high water mark during development
- Avoid large local variables (use heap or static)
- Limit recursion depth
- Use appropriate stack sizes for tasks
- Enable stack monitoring in development builds

## Related
- FreeRTOS Task Management
- Stack Overflow Detection
- Memory Management
