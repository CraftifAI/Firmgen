# C2000 Development Tools for Refact Agent

This directory contains C2000-specific tools that extend the Refact agent with TI C2000 microcontroller development capabilities.

## Overview

The C2000 tools transform complex CCS CLI workflows into natural language interactions, allowing users to describe what they want to accomplish rather than memorizing commands.

## Tools Available

### 1. **Project Management**
- **`ToolC2000ProjectCreate`** - Create CCS projects from .projectspec files
- **`ToolC2000ConfigValidate`** - Validate project configurations and settings

### 2. **Build & Flash**
- **`ToolC2000Build`** - Build projects for specific configurations (RAM/FLASH)
- **`ToolC2000Flash`** - Flash devices using DSLite with verification

### 3. **Debug & Monitoring**
- **`ToolC2000UartCapture`** - Capture UART output with intelligent analysis
- **`ToolC2000TargetDetect`** - Detect connected target devices and debug probes

### 4. **Example Management**
- **`ToolC2000ExampleList`** - List available C2000Ware examples with filtering

## Configuration

Create `~/.cache/refact/c2000_tools.yaml` based on `sample_config.yaml`:

```yaml
c2000_config:
  ccs_path: "/home/user/ti/ccs2020/ccs"
  workspace_path: "/home/user/ti/ccs2020/ccs/example_workspace"
  c2000ware_path: "/home/user/ti/C2000Ware_6_00_00_00"
  default_uart_device: "/dev/ttyACM0"
  default_uart_baud: 115200
  default_uart_parity: "odd"

tools:
  c2000_project_create:
    enabled: true
  # ... other tools
```

## Usage Examples

### Natural Language Interactions

**Before (Command-based):**
```bash
$CCS/eclipse/ccs-server-cli.sh -workspace "$WS" \
  -application projectCreate \
  -ccs.projectSpec "$C2000WARE/device_support/f28p65x/examples/cpu1/spi/CCS/spi_ex1_loopback.projectspec" \
  -ccs.renameTo spi_ex1_loopback -ccs.copyIntoWorkspace
```

**After (Agent tool-based):**
```
User: "Create a SPI loopback project for F28P65x LaunchPad"

Agent: ✅ Project 'spi_ex1_loopback' created successfully
      📁 Location: /home/user/ti/ccs2020/ccs/example_workspace/spi_ex1_loopback
```

### Complete Workflow Example

```
User: "Create a SPI loopback project, build it for FLASH, flash it, and capture UART output for 10 seconds"

Agent: I'll help you set up a complete SPI loopback test:

1. [c2000_project_create] Creating SPI loopback project...
   ✅ Project 'spi_ex1_loopback' created successfully

2. [c2000_build] Building for CPU1_FLASH configuration...
   ✅ Build completed successfully

3. [c2000_flash] Programming target device...
   ✅ Flash programming completed
   ✅ Verification passed

4. [c2000_uart_capture] Starting UART capture...
   📡 Monitoring /dev/ttyACM0 at 115200 baud
   ⏱️ Capturing for 10 seconds...
   ✅ Capture completed
   📊 Analysis: SPI communication detected, loopback test passed
```

## Integration

To integrate these tools into the Refact agent:

1. **Add to `tools/mod.rs`:**
```rust
pub mod c2000_tools;
```

2. **Add to `tools_list.rs`:**
```rust
let c2000_tools: Vec<Box<dyn Tool + Send>> = vec![
    Box::new(crate::tools::c2000_tools::ToolC2000ProjectCreate{config_path: config_path.clone()}),
    Box::new(crate::tools::c2000_tools::ToolC2000Build{config_path: config_path.clone()}),
    // ... other tools
];

// Add to tool_groups
ToolGroup {
    name: "C2000 Development".to_string(),
    description: "TI C2000 microcontroller development tools".to_string(),
    category: ToolGroupCategory::Builtin,
    tools: c2000_tools,
},
```

## Dependencies

- **CCS (Code Composer Studio)** - Required for project creation and building
- **C2000Ware** - Required for example projects and libraries
- **DSLite** - Required for flashing (included with CCS)
- **minicom** - Required for UART capture (Linux)
- **walkdir** - Rust crate for directory traversal

## Error Handling

All tools include comprehensive error handling:
- Configuration validation
- Path existence checks
- Command execution error reporting
- Intelligent error messages with suggestions

## Benefits

✅ **Natural Language Interface** - Describe what you want instead of memorizing commands  
✅ **Intelligent Orchestration** - Agent chains multiple operations automatically  
✅ **Error Handling** - Smart diagnosis and suggestions for common issues  
✅ **Integration** - Works seamlessly with other Refact tools  
✅ **Extensibility** - Easy to add new C2000-specific capabilities  

## File Structure

```
c2000_tools/
├── mod.rs                    # Module declarations and re-exports
├── config.rs                 # Configuration management
├── project_create.rs         # Project creation tool
├── build.rs                  # Build tool
├── flash.rs                  # Flash programming tool
├── uart_capture.rs           # UART capture tool
├── config_validate.rs        # Configuration validation tool
├── target_detect.rs          # Target detection tool
├── example_list.rs           # Example listing tool
├── sample_config.yaml        # Sample configuration file
└── README.md                 # This file
```

## Next Steps

1. **Review the tools** - Check implementations and modify as needed
2. **Test integration** - Add to main tools module and test
3. **Add advanced features** - Implement log analysis, automatic troubleshooting
4. **Create documentation** - Add user guides and examples
5. **Extend functionality** - Add support for more C2000 devices and peripherals






