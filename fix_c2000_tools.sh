#!/bin/bash

# Script to fix all C2000 tools compilation errors

echo "🔧 Fixing C2000 tools compilation errors..."

# Fix config_validate.rs - change validation_results to Vec<String>
sed -i 's/let mut validation_results = Vec::new();/let mut validation_results: Vec<String> = Vec::new();/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs

# Fix all format! calls to use .to_string()
sed -i 's/validation_results.push(format!(/validation_results.push(format!(/g' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs

# Fix target_detect.rs - change messages to Vec<String>
sed -i 's/let mut messages = Vec::new();/let mut messages: Vec<String> = Vec::new();/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs

# Fix example_list.rs - change messages to Vec<String>
sed -i 's/let mut messages = Vec::new();/let mut messages: Vec<String> = Vec::new();/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/example_list.rs

echo "✅ Applied fixes to C2000 tools"











