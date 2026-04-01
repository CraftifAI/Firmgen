# C2000 Project Architecture Overview

## System Architecture

### Core Components
1. **Refact Agent Engine**: Rust-based backend with native C2000 tools
2. **Refact Server**: Python-based API server with C2000 configuration
3. **Refact GUI**: React-based frontend for visualization and interaction
4. **C2000 Native Tools**: 8 specialized Rust tools for C2000 development

### Native Tools Workflow

#### Core Development Tools
- **c2000_project_create**: Creates CCS projects from C2000Ware examples
- **c2000_build**: Compiles projects with various configurations
- **c2000_flash**: Programs firmware to target hardware
- **c2000_uart_capture**: Captures UART output for monitoring

#### Diagnostic Tools
- **c2000_target_detect**: Detects and verifies hardware connections
- **c2000_example_list**: Lists available C2000Ware examples
- **c2000_config_validate**: Validates C2000 configuration and paths

#### AI-Powered Analysis
- **c2000_code_evaluator**: AI-powered code evaluation using Refact's internal LLM

## Configuration System

### Dynamic Configuration
- **Primary**: HTTP API endpoint (`http://localhost:8002/v1/c2000-config`)
- **Fallback**: File-based configuration (`~/.cache/refact/c2000_tools.yaml`)
- **Real-time updates**: Configuration changes without binary recompilation

### Key Configuration Parameters
```yaml
c2000_config:
  ccs_path: "/home/shubham/ti/ccs2020/ccs"
  c2000ware_path: "/home/shubham/ti/C2000Ware_6_00_00_00"
  workspace_path: "/home/shubham/ti/ccs_workspace"
  target_device: "F28P65x"
  debug_probe: "XDS110"
```

## Hardware Platform

### Target Hardware
- **MCU**: TMS320F28P65x (C2000 Real-time MCU)
- **Board**: LAUNCHXL-F28P65X LaunchPad
- **Debug Probe**: XDS110/XDS100v2 JTAG debug probe
- **Clock**: 200 MHz system clock
- **Memory**: 1MB Flash, 256KB RAM

### Development Environment
- **IDE**: Code Composer Studio (CCS) 2020
- **Compiler**: TI cl2000 (C2000-specific)
- **SDK**: C2000Ware SDK 6.00.00.00
- **Build System**: gmake with CCS integration
- **Configuration**: SysConfig GUI for peripheral setup

## Software Stack

### SDK Components
- **DriverLib**: High-level peripheral APIs
- **Device Support**: Low-level device definitions
- **TI-RTOS**: Real-time operating system
- **SysConfig**: Graphical configuration tool

### Build Configurations
- **CPU1_LAUNCHXL_RAM**: Development and testing
- **CPU1_LAUNCHXL_FLASH**: Production deployment
- **CPU1_RAM**: Generic RAM configuration
- **CPU1_FLASH**: Generic FLASH configuration

## Communication Interfaces

### Debug Interface
- **Protocol**: JTAG/SWD
- **Probe**: XDS110/XDS100v2
- **GDB Server**: CCS integrated debugger
- **Port**: 3333 (default)

### Serial Communication
- **Interface**: SCI-A (UART)
- **Baud Rate**: 115200
- **Port**: /dev/ttyUSB0 (Linux)
- **Purpose**: Logging and monitoring

## Project Structure

### Repository Layout
```
refact/
├── refact-agent/engine/          # Rust backend with C2000 tools
├── refact-server/                # Python API server
├── refact-diagram/               # React frontend
├── docs/                         # Documentation
├── .embedder/                    # Embedder configuration
│   ├── documents/                # Project documentation
│   └── EMBEDDER.md              # Main configuration
└── caps.json                     # Model configuration
```

### C2000 Tools Integration
- **Rust Implementation**: Native performance and safety
- **HTTP API**: RESTful interface for tool invocation
- **Configuration**: Dynamic configuration management
- **Error Handling**: Comprehensive error reporting and recovery

## Development Workflow

### 1. Project Discovery
- Use `c2000_example_list` to find relevant examples
- Search C2000Ware SDK for specific peripherals or use cases

### 2. Project Creation
- Use `c2000_project_create` to instantiate projects
- Configure for target device (F28P65x)
- Set up build configurations

### 3. Development Cycle
- Use `c2000_build` for compilation
- Use `c2000_flash` for programming
- Use `c2000_uart_capture` for monitoring

### 4. Hardware Verification
- Use `c2000_target_detect` for connection verification
- Use `c2000_config_validate` for configuration validation

### 5. Code Analysis
- Use `c2000_code_evaluator` for AI-powered code review
- Leverage Refact's LLM for code suggestions and improvements

## Integration Points

### CCS Integration
- Projects created in CCS workspace
- Build system integration with gmake
- Debugger integration with XDS probes
- SysConfig integration for peripheral configuration

### Refact Integration
- Native tool compilation into Refact binary
- HTTP API for tool invocation
- Configuration management through Refact server
- AI-powered code analysis and suggestions
