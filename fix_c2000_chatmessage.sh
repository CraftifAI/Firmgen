#!/bin/bash

# Fix all C2000 tools to use correct ChatMessage structure

echo "Fixing C2000 tools ChatMessage structure..."

# Function to fix a file
fix_file() {
    local file="$1"
    echo "Fixing $file..."
    
    # Replace the old ChatMessage structure with the correct one
    sed -i 's/finish_reason: None,//g' "$file"
    sed -i 's/tool_failed: None,//g' "$file"
    sed -i 's/usage: None,//g' "$file"
    sed -i 's/checkpoints: vec!\[\],//g' "$file"
    sed -i 's/thinking_blocks: None,//g' "$file"
    
    # Replace the pattern with ..Default::default()
    sed -i 's/tool_call_id: tool_call_id\.clone(),$/tool_call_id: tool_call_id.clone(),\n            ..Default::default()/g' "$file"
}

# Fix all C2000 tool files
fix_file "refact-agent/engine/src/tools/c2000_tools/project_create.rs"
fix_file "refact-agent/engine/src/tools/c2000_tools/build.rs"
fix_file "refact-agent/engine/src/tools/c2000_tools/flash.rs"
fix_file "refact-agent/engine/src/tools/c2000_tools/uart_capture.rs"
fix_file "refact-agent/engine/src/tools/c2000_tools/config_validate.rs"
fix_file "refact-agent/engine/src/tools/c2000_tools/target_detect.rs"

echo "Done fixing C2000 tools!"











