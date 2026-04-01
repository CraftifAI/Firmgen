#!/bin/bash
# C2000 Development Helper Script
# This script provides common C2000 operations for use with Refact

# Environment check
check_env() {
    echo "=== C2000 Environment Check ==="
    echo "CCS: $CCS"
    echo "C2000WARE: $C2000WARE"
    echo "WS: $WS"
    echo "==============================="
}

# List available examples
list_examples() {
    echo "=== Available C2000Ware Examples ==="
    find $C2000WARE/examples -name "*.projectspec" | head -20
    echo "====================================="
}

# Create project
create_project() {
    local project_name=$1
    local example_name=$2
    
    if [ -z "$project_name" ] || [ -z "$example_name" ]; then
        echo "Usage: create_project <project_name> <example_name>"
        echo "Example: create_project my_spi_test spi_loopback"
        return 1
    fi
    
    echo "Creating project '$project_name' from example '$example_name'..."
    $CCS/eclipse/ccs-server-cli.sh -application projectCreate \
        -ccs.projectSpec $C2000WARE/examples/$example_name/$example_name.projectspec \
        -ccs.renameTo $project_name \
        -ccs.copyIntoWS $WS
    
    if [ $? -eq 0 ]; then
        echo "✅ Project '$project_name' created successfully!"
        echo "Location: $WS/$project_name"
    else
        echo "❌ Failed to create project '$project_name'"
        return 1
    fi
}

# Build project
build_project() {
    local project_name=$1
    local config=${2:-Debug}
    
    if [ -z "$project_name" ]; then
        echo "Usage: build_project <project_name> [config]"
        echo "Config options: Debug, Release, RAM, FLASH"
        return 1
    fi
    
    echo "Building project '$project_name' with '$config' configuration..."
    $CCS/eclipse/ccs-server-cli.sh -application projectBuild \
        -ccs.projects $WS/$project_name \
        -ccs.config $config
    
    if [ $? -eq 0 ]; then
        echo "✅ Project '$project_name' built successfully!"
        echo "Output files:"
        find $WS/$project_name -name "*.out" -exec ls -la {} \;
    else
        echo "❌ Failed to build project '$project_name'"
        return 1
    fi
}

# Detect target
detect_target() {
    local project_name=$1
    local ccxml_file=${2:-TMS320F28P650DK9.ccxml}
    
    if [ -z "$project_name" ]; then
        echo "Usage: detect_target <project_name> [ccxml_file]"
        return 1
    fi
    
    echo "Detecting target for project '$project_name'..."
    dslite -c $WS/$project_name/targetConfigs/$ccxml_file -e -v
    
    if [ $? -eq 0 ]; then
        echo "✅ Target detected successfully!"
    else
        echo "❌ Failed to detect target"
        return 1
    fi
}

# Flash project
flash_project() {
    local project_name=$1
    local ccxml_file=${2:-TMS320F28P650DK9.ccxml}
    
    if [ -z "$project_name" ]; then
        echo "Usage: flash_project <project_name> [ccxml_file]"
        return 1
    fi
    
    echo "Flashing project '$project_name'..."
    dslite -c $WS/$project_name/targetConfigs/$ccxml_file -e -f -v -n 0 -r 0
    
    if [ $? -eq 0 ]; then
        echo "✅ Project '$project_name' flashed successfully!"
    else
        echo "❌ Failed to flash project '$project_name'"
        return 1
    fi
}

# Capture UART
capture_uart() {
    local device=${1:-/dev/ttyACM0}
    local baud=${2:-115200}
    local duration=${3:-30}
    local log_file=${4:-/tmp/uart_capture.log}
    
    echo "Capturing UART from $device at $baud baud for $duration seconds..."
    echo "Output will be saved to: $log_file"
    
    timeout $duration minicom -D $device -b $baud -C $log_file
    
    if [ $? -eq 0 ]; then
        echo "✅ UART capture completed!"
        echo "Log file: $log_file"
        echo "Last 10 lines:"
        tail -10 $log_file
    else
        echo "❌ UART capture failed or timed out"
        return 1
    fi
}

# Check project status
project_status() {
    local project_name=$1
    
    if [ -z "$project_name" ]; then
        echo "Usage: project_status <project_name>"
        return 1
    fi
    
    echo "=== Project Status: $project_name ==="
    if [ -d "$WS/$project_name" ]; then
        echo "✅ Project exists"
        echo "Directory contents:"
        ls -la $WS/$project_name
        echo ""
        echo "Build outputs:"
        find $WS/$project_name -name "*.out" -exec ls -la {} \; 2>/dev/null || echo "No .out files found"
    else
        echo "❌ Project does not exist"
        return 1
    fi
    echo "====================================="
}

# List UART devices
list_uart_devices() {
    echo "=== Available UART Devices ==="
    ls -la /dev/tty* | grep -E '(ACM|USB)'
    echo ""
    echo "Recent UART activity:"
    dmesg | tail -10 | grep -i 'tty' || echo "No recent UART activity"
    echo "============================="
}

# Evaluate code
evaluate_code() {
    local golden_file=$1
    local candidate_file=$2
    local model=${3:-gpt-4.1}
    
    if [ -z "$golden_file" ] || [ -z "$candidate_file" ]; then
        echo "Usage: evaluate_code <golden_file> <candidate_file> [model]"
        echo "Example: evaluate_code /path/to/golden.c /path/to/candidate.c gpt-4.1"
        return 1
    fi
    
    echo "Evaluating code..."
    echo "Golden: $golden_file"
    echo "Candidate: $candidate_file"
    echo "Model: $model"
    
    python3 /home/shubham/sdk_agent/refact/evaluate_patch_3.py \
        --golden "$golden_file" \
        --candidate "$candidate_file" \
        --model "$model"
}

# Main function
main() {
    case "$1" in
        "check_env")
            check_env
            ;;
        "list_examples")
            list_examples
            ;;
        "create_project")
            create_project "$2" "$3"
            ;;
        "build_project")
            build_project "$2" "$3"
            ;;
        "detect_target")
            detect_target "$2" "$3"
            ;;
        "flash_project")
            flash_project "$2" "$3"
            ;;
        "capture_uart")
            capture_uart "$2" "$3" "$4" "$5"
            ;;
        "project_status")
            project_status "$2"
            ;;
        "list_uart_devices")
            list_uart_devices
            ;;
        "evaluate_code")
            evaluate_code "$2" "$3" "$4"
            ;;
        *)
            echo "C2000 Development Helper Script"
            echo ""
            echo "Available commands:"
            echo "  check_env                    - Check environment variables"
            echo "  list_examples                - List available C2000Ware examples"
            echo "  create_project <name> <example> - Create new project from example"
            echo "  build_project <name> [config]   - Build project (Debug/Release/RAM/FLASH)"
            echo "  detect_target <name> [ccxml]    - Detect target connection"
            echo "  flash_project <name> [ccxml]    - Flash project to target"
            echo "  capture_uart [device] [baud] [duration] [log] - Capture UART output"
            echo "  project_status <name>           - Check project status"
            echo "  list_uart_devices              - List available UART devices"
            echo "  evaluate_code <golden> <candidate> [model] - Evaluate code with AI"
            echo ""
            echo "Examples:"
            echo "  $0 create_project my_spi_test spi_loopback"
            echo "  $0 build_project my_spi_test Debug"
            echo "  $0 flash_project my_spi_test"
            echo "  $0 capture_uart /dev/ttyACM0 115200 30"
            ;;
    esac
}

# Run main function with all arguments
main "$@"












