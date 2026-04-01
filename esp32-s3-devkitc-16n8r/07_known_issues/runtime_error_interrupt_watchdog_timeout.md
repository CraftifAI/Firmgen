# Runtime Error: Interrupt Watchdog Timeout

## Error Pattern
```
Guru Meditation Error: Core 0 panic'ed (Interrupt wdt timeout on CPU0).
Interrupt watchdog timeout on CPU0
```

## Issue Type
**runtime_error**, **panic**

## Component
**esp_system**, **interrupt_wdt**

## Root Cause
An interrupt service routine (ISR) is taking too long to execute, or a blocking operation is being performed in an ISR.

## Common Causes

1. **ISR too long**: ISR processing takes longer than watchdog timeout
2. **Blocking in ISR**: Calling blocking functions (malloc, printf, etc.) in ISR
3. **Nested interrupts**: Too many nested interrupts
4. **Watchdog timeout too short**: CONFIG_ESP_INT_WDT_TIMEOUT_MS too low

## Solutions

### 1. Reduce ISR Processing Time
```c
// BAD - Too much work in ISR
void IRAM_ATTR my_isr_handler(void *arg) {
    // Lots of processing...
    process_data();
    send_data();
    update_display();
    // Takes too long!
}

// GOOD - Minimal work, defer to task
void IRAM_ATTR my_isr_handler(void *arg) {
    BaseType_t xHigherPriorityTaskWoken = pdFALSE;
    // Just notify task
    xSemaphoreGiveFromISR(semaphore, &xHigherPriorityTaskWoken);
    portYIELD_FROM_ISR(xHigherPriorityTaskWoken);
}
```

### 2. Move Blocking Operations to Task
```c
// BAD - Blocking in ISR
void IRAM_ATTR my_isr_handler(void *arg) {
    printf("Interrupt occurred\n");  // Blocking!
    malloc(100);  // Blocking!
}

// GOOD - Defer to task
void IRAM_ATTR my_isr_handler(void *arg) {
    // Set flag or send notification
    interrupt_flag = true;
    xTaskNotifyFromISR(task_handle, 0, eNoAction, NULL);
}
```

### 3. Increase Watchdog Timeout
```c
// In menuconfig
CONFIG_ESP_INT_WDT_TIMEOUT_MS=800  // Default 800ms, increase if needed
```

**Note**: Increasing timeout is a workaround. Better to fix the ISR.

### 4. Disable Watchdog (Not Recommended)
```c
// Only for debugging
CONFIG_ESP_INT_WDT=n
```

**Warning**: Disabling watchdog can cause system hangs. Not recommended for production.

### 5. Use IRAM-Safe Functions
```c
// Functions that can be called from ISR
// Must be in IRAM and not use flash cache

// Register ISR with IRAM flag
esp_intr_alloc(..., ESP_INTR_FLAG_IRAM, ...);

// ISR code must be in IRAM
void IRAM_ATTR my_isr_handler(void *arg) {
    // Code here must be in IRAM
    // Cannot call functions that access flash
}
```

## ESP32-S3-DevKitC Specific

- **Default timeout**: 800ms
- **Dual-core**: Each core has its own interrupt watchdog
- **IRAM requirements**: ISR code must be in IRAM if flash cache disabled

## Debugging

1. **Identify ISR**: Check which interrupt triggered timeout
2. **Review ISR code**: Look for blocking operations
3. **Check ISR duration**: Measure time spent in ISR
4. **Review recent changes**: What ISR code was modified?

## Best Practices

- Keep ISRs short (< 100µs ideally)
- Defer processing to tasks
- Use queues/semaphores for ISR-to-task communication
- Never call blocking functions in ISR
- Use IRAM for time-critical ISRs
- Test ISR performance

## Related
- Interrupt Allocation API
- IRAM-Safe Interrupt Handlers
- Watchdog documentation
