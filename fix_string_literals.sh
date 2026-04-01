#!/bin/bash

# Fix all string literal issues in C2000 tools

echo "🔧 Fixing all string literal issues..."

# Fix config_validate.rs
sed -i 's/validation_results.push("✅ C2000Ware integration available");/validation_results.push("✅ C2000Ware integration available".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs
sed -i 's/validation_results.push("❌ C2000Ware path not accessible");/validation_results.push("❌ C2000Ware path not accessible".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs
sed -i 's/validation_results.push("✅ CCS CLI available");/validation_results.push("✅ CCS CLI available".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs
sed -i 's/validation_results.push("❌ CCS CLI not found");/validation_results.push("❌ CCS CLI not found".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs
sed -i 's/validation_results.push("✅ DSLite available");/validation_results.push("✅ DSLite available".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs
sed -i 's/validation_results.push("❌ DSLite not found");/validation_results.push("❌ DSLite not found".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/config_validate.rs

# Fix target_detect.rs
sed -i 's/messages.push("🎯 Detected Target Configurations:");/messages.push("🎯 Detected Target Configurations:".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs
sed -i 's/messages.push("⚠️ No target configurations found in workspace");/messages.push("⚠️ No target configurations found in workspace".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs
sed -i 's/messages.push("🔧 Available Cores:");/messages.push("🔧 Available Cores:".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs
sed -i 's/messages.push("⚠️ Could not list cores (device may not be connected)");/messages.push("⚠️ Could not list cores (device may not be connected)".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs
sed -i 's/messages.push("🔄 Available Reset Actions:");/messages.push("🔄 Available Reset Actions:".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs
sed -i 's/messages.push("⚠️ Could not list reset actions");/messages.push("⚠️ Could not list reset actions".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs
sed -i 's/messages.push("❌ DSLite not found - cannot detect connected devices");/messages.push("❌ DSLite not found - cannot detect connected devices".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs
sed -i 's/messages.push("🔌 Detected USB Devices:");/messages.push("🔌 Detected USB Devices:".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs

# Fix example_list.rs
sed -i 's/messages.push("");/messages.push("".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/example_list.rs
sed -i 's/messages.push("❌ No examples found matching the specified criteria");/messages.push("❌ No examples found matching the specified criteria".to_string());/' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/example_list.rs

echo "✅ Fixed all string literal issues"











