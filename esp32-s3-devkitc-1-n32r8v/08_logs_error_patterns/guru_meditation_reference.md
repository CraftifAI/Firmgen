# Guru Meditation Error Reference

## Overview

"Guru Meditation Error" is ESP-IDF's panic handler message for fatal errors. The name comes from the Amiga computer's error message tradition.

## Error Format

```
Guru Meditation Error: Core 0 panic'ed (ErrorType). Exception was unhandled.
```

## Common Error Types

### CPU Exceptions (RISC-V Architecture - ESP32-S3)

#### IllegalInstruction
- **Cause**: Invalid instruction executed
- **Common reasons**: FreeRTOS task returned, SPI flash pins reconfigured, C++ missing return
- **Fix**: Use `vTaskDelete(NULL)`, don't reconfigure GPIO26-32, always return from non-void functions

#### Instruction Access Fault
- **Cause**: Attempted to execute from invalid memory
- **Common reasons**: Function pointer is NULL or invalid
- **Fix**: Check function pointers before calling, verify code is in valid memory region

#### Load Access Fault
- **Cause**: Attempted to read from invalid memory
- **Common reasons**: NULL pointer dereference, uninitialized pointer, corrupted pointer
- **Fix**: Check EXCVADDR (MTVAL on RISC-V) - 0x0 = NULL pointer, garbage = corrupted

#### Store Access Fault
- **Cause**: Attempted to write to invalid memory
- **Common reasons**: NULL pointer write, write to read-only memory
- **Fix**: Check pointer validity, verify memory is writable

#### Load Address Misaligned / Store Address Misaligned
- **Cause**: Memory access not properly aligned
- **Common reasons**: Unaligned pointer arithmetic, structure packing issues
- **Fix**: Ensure pointers are properly aligned (4-byte for 32-bit, 2-byte for 16-bit)

#### Breakpoint
- **Cause**: EBREAK instruction executed
- **Common reasons**: Stack overflow detection, debug breakpoint
- **Fix**: Check for stack overflow, review breakpoint usage

### System Level Errors

#### Interrupt wdt timeout on CPU0/CPU1
- **Cause**: ISR took too long (>800ms default)
- **Fix**: Reduce ISR processing time, move blocking operations to tasks

#### Cache access error
- **Cause**: Cache access failed during flash/PSRAM operation
- **Fix**: Ensure ISR handlers are IRAM-safe, check flash cache configuration

#### Stack protection fault
- **Cause**: Stack overflow detected by hardware
- **Fix**: Increase task stack size, check for infinite recursion

#### Brownout detector was triggered
- **Cause**: Supply voltage dropped below safe level
- **Fix**: Improve power supply, reduce current draw, adjust brownout level

## Register Dump Interpretation

### RISC-V (ESP32-S3) Registers

- **MEPC**: Program counter where exception occurred
- **RA**: Return address
- **SP**: Stack pointer
- **MTVAL**: Fault address (for Load/Store Access Fault)
- **MCAUSE**: Exception cause code

### Key Address Ranges

- **0x3FC00000 - 0x3FFFFFFF**: Internal DRAM
- **0x40000000 - 0x4007FFFF**: Internal IRAM
- **0x42000000+**: Flash memory-mapped
- **0x3D000000+**: PSRAM memory-mapped (if enabled)

## Backtrace Decoding

### With IDF Monitor (Automatic)
```
Backtrace: 0x420048b4:0x3fc8f2f0 0x420048b4:0x3fc8f2f0
0x420048b4: app_main at /path/to/main.c:20
```

### Manual Decoding
```bash
xtensa-esp32s3-elf-addr2line -pfiaC -e build/app.elf 0x420048b4
```

## Debugging Workflow

1. **Read error type**: Identify the exception from panic message
2. **Check register dump**: Look at MEPC (PC), MTVAL (fault address), SP (stack)
3. **Decode backtrace**: Use addr2line or IDF monitor
4. **Check EXCVADDR/MTVAL**: 
   - 0x0 = NULL pointer
   - Close to 0 = Structure member access with NULL pointer
   - Garbage = Uninitialized or corrupted pointer
5. **Review recent changes**: What code was added/modified before crash?

## Prevention

- Always initialize pointers
- Check return values from malloc/calloc
- Use ESP_ERROR_CHECK() for error handling
- Enable stack overflow detection
- Keep ISRs short
- Don't reconfigure SPI flash pins
- Use appropriate stack sizes

## Related
- Fatal Errors documentation
- Error Handling guide
- Panic Handler configuration
