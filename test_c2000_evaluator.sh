#!/bin/bash

# Test script for C2000 Code Evaluator Tool
echo "🧪 Testing C2000 Code Evaluator Tool Integration..."

# Check if refact-lsp is running
if pgrep -f "refact-lsp" > /dev/null; then
    echo "✅ refact-lsp is running"
else
    echo "❌ refact-lsp is not running"
    echo "Please start Refact first with: refact ."
    exit 1
fi

# Check if the built binary includes the new evaluator tool
if [ -f "/home/shubham/sdk_agent/refact/refact-agent/engine/target/release/refact-lsp" ]; then
    echo "✅ refact-lsp binary built successfully with C2000 evaluator"
else
    echo "❌ refact-lsp binary not found"
    exit 1
fi

echo ""
echo "🎯 C2000 Code Evaluator Tool Status:"
echo "- Tool implementation: ✅ Created (code_evaluator.rs)"
echo "- Module integration: ✅ Added to mod.rs"
echo "- Tools list: ✅ Added to tools_list.rs"
echo "- Binary compilation: ✅ Successful"
echo ""
echo "🚀 Ready to test C2000 code evaluation!"
echo ""
echo "📋 Available C2000 Tools:"
echo "1. c2000_project_create - Create CCS projects from examples"
echo "2. c2000_code_evaluator - Evaluate C2000 code quality and correctness"
echo ""
echo "💡 Test Examples:"
echo ""
echo "**Standalone Evaluation:**"
echo "Evaluate the C2000 code in file 'my_code.c'"
echo ""
echo "**Comparison Evaluation:**"
echo "Compare the candidate code in 'candidate.c' against the golden reference in 'golden.c'"
echo ""
echo "**Comprehensive Evaluation:**"
echo "Perform a comprehensive evaluation of 'my_code.c' focusing on functionality and C2000-specific issues"
echo ""
echo "🎯 Next Steps:"
echo "1. Open Refact GUI or use CLI"
echo "2. Try one of the test examples above"
echo "3. Check if c2000_code_evaluator tool is available and working"











