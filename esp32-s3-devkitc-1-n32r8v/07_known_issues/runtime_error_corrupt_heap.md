# Runtime Error: Corrupt Heap

## Error Pattern
```
CORRUPT HEAP: Bad tail at 0x3ffe270a. Expected 0xbaad5678 got 0xbaac5678
assertion "head != NULL" failed: file ".../multi_heap_poisoning.c", line 201
abort() was called at PC 0x400dca43 on core 0
```

## Issue Type
**runtime_error**, **panic**

## Component
**heap**

## Root Cause
Heap structure has been corrupted, typically due to:
1. Buffer overrun (writing past allocated memory)
2. Use-after-free (accessing freed memory)
3. Double-free (freeing same pointer twice)
4. Freeing invalid pointer (not from malloc/calloc)

## Solutions

### 1. Enable Heap Poisoning
```c
// In menuconfig
CONFIG_HEAP_POISONING_COMPREHENSIVE=y
// or
CONFIG_HEAP_POISONING_LIGHT=y
```

This will help detect corruption earlier.

### 2. Enable Heap Tracing
```c
// In menuconfig
CONFIG_HEAP_TRACING_DEST_NONE=n
CONFIG_HEAP_TRACING_DEST_HOST=y  // or _HEAP_TRACE_DEST_UART0

// In code
#include "esp_heap_trace.h"

heap_trace_init_standalone(trace_record, NUM_RECORDS);
heap_trace_start(HEAP_TRACE_LEAKS);
// ... your code ...
heap_trace_stop();
heap_trace_dump();
```

### 3. Check for Buffer Overruns
```c
// BAD - Buffer overrun
char buffer[10];
strcpy(buffer, "This string is too long!");  // Overrun!

// GOOD - Check bounds
char buffer[10];
if (strlen(str) < sizeof(buffer)) {
    strcpy(buffer, str);
} else {
    ESP_LOGE(TAG, "String too long");
}
```

### 4. Avoid Use-After-Free
```c
// BAD - Use after free
void *ptr = malloc(100);
free(ptr);
*ptr = 10;  // CRASH! Using freed memory

// GOOD - Set to NULL after free
void *ptr = malloc(100);
free(ptr);
ptr = NULL;  // Prevents accidental use
```

### 5. Avoid Double-Free
```c
// BAD - Double free
void *ptr = malloc(100);
free(ptr);
free(ptr);  // CRASH! Freeing twice

// GOOD - Set to NULL after free
void *ptr = malloc(100);
free(ptr);
ptr = NULL;
free(ptr);  // Safe (free(NULL) is no-op)
```

### 6. Use Static Analysis
```bash
# Enable compiler sanitizers (if available)
CONFIG_COMPILER_SAVE_RESTORE_LIBCALLS=y
```

## Debugging Steps

1. **Enable heap poisoning**: CONFIG_HEAP_POISONING_COMPREHENSIVE
2. **Enable heap tracing**: Track all allocations/frees
3. **Check backtrace**: Identify which function corrupted heap
4. **Review memory operations**: Look for buffer operations near crash
5. **Use GDB**: Attach debugger to inspect heap state

## Common Patterns

### Pattern 1: Array Index Out of Bounds
```c
int array[10];
array[15] = 100;  // Overrun!
```

### Pattern 2: String Operations
```c
char str[10];
sprintf(str, "Very long string that exceeds buffer");  // Overrun!
```

### Pattern 3: Structure Member Access
```c
struct my_struct *s = malloc(sizeof(struct my_struct));
free(s);
s->member = 10;  // Use-after-free
```

## Prevention

- Always check buffer bounds
- Use safe string functions (strncpy, snprintf)
- Set pointers to NULL after free
- Enable heap poisoning in development
- Use static analysis tools
- Review code for memory safety

## Related
- Heap Memory Debugging documentation
- Memory Management
- Error Handling
