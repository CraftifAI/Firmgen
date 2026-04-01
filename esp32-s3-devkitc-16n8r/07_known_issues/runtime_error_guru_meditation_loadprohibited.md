# Runtime Error: Guru Meditation - LoadProhibited

## Error Pattern
```
Guru Meditation Error: Core 0 panic'ed (LoadProhibited). Exception was unhandled.
Core 0 register dump:
EXCVADDR: 0x00000000  (or other address)
```

## Issue Type
**runtime_error**, **panic**

## Component
**esp_system**, **panic_handler**

## Root Cause
Application attempted to read from an invalid memory location. The address in `EXCVADDR` register indicates where the fault occurred.

## Diagnosis by EXCVADDR Value

### EXCVADDR = 0x00000000 (or close to zero)
**NULL pointer dereference**
- Most common cause
- Pointer was not initialized or set to NULL
- Accessing structure member when structure pointer is NULL

### EXCVADDR = garbage value (not in 0x3fxxxxxx - 0x6xxxxxxx range)
**Uninitialized or corrupted pointer**
- Pointer was never initialized
- Pointer was corrupted (stack overflow, buffer overrun)
- Pointer points to freed memory (use-after-free)

## Solutions

### For NULL Pointer (EXCVADDR = 0x00000000)
1. **Check pointer initialization**:
   ```c
   // BAD
   int *ptr;
   *ptr = 10;  // Crash!
   
   // GOOD
   int *ptr = malloc(sizeof(int));
   if (ptr != NULL) {
       *ptr = 10;
   }
   ```

2. **Check function return values**:
   ```c
   wifi_config_t *config = malloc(sizeof(wifi_config_t));
   if (config == NULL) {
       ESP_LOGE(TAG, "Failed to allocate memory");
       return ESP_ERR_NO_MEM;
   }
   ```

3. **Check structure member access**:
   ```c
   // BAD
   my_struct_t *s = NULL;
   s->member = 10;  // Crash!
   
   // GOOD
   if (s != NULL) {
       s->member = 10;
   }
   ```

### For Corrupted Pointer
1. **Enable stack overflow detection**: `CONFIG_ESP_SYSTEM_HW_STACK_GUARD`
2. **Check for buffer overruns**: Use static analysis tools
3. **Enable heap debugging**: `CONFIG_HEAP_POISONING_COMPREHENSIVE`
4. **Review backtrace**: Identify which function corrupted the pointer

## Debugging Steps

1. **Check backtrace**: Use `idf.py monitor` to see decoded addresses
2. **Enable core dump**: `CONFIG_ESP_COREDUMP_ENABLE_TO_FLASH`
3. **Use GDB Stub**: `CONFIG_ESP_SYSTEM_PANIC_GDBSTUB`
4. **Check stack usage**: `CONFIG_FREERTOS_GENERATE_RUN_TIME_STATS`

## Prevention

- Always initialize pointers
- Always check return values from malloc/calloc
- Use `ESP_ERROR_CHECK()` for error handling
- Enable compiler warnings: `-Wall -Wextra`
- Use static analysis tools

## Related
- Fatal Errors documentation
- Error Handling guide
- Heap Memory Debugging
