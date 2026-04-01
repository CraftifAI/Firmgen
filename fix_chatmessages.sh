#!/bin/bash

# Quick fix for remaining ChatMessage issues

echo "🔧 Fixing remaining ChatMessage issues..."

# Fix target_detect.rs ChatMessage issues
sed -i 's/tool_calls: None,/finish_reason: None,\n            tool_calls: None,\n            tool_call_id: tool_call_id.clone(),\n            tool_failed: None,\n            usage: None,\n            checkpoints: vec![],\n            thinking_blocks: None,/g' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/target_detect.rs

# Fix example_list.rs ChatMessage issues  
sed -i 's/tool_calls: None,/finish_reason: None,\n            tool_calls: None,\n            tool_call_id: tool_call_id.clone(),\n            tool_failed: None,\n            usage: None,\n            checkpoints: vec![],\n            thinking_blocks: None,/g' /home/shubham/sdk_agent/refact/refact-agent/engine/src/tools/c2000_tools/example_list.rs

echo "✅ Fixed ChatMessage issues"











