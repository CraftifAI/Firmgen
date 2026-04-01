import { motion } from "framer-motion";
import { Brain, Code, FileText, Settings, Terminal, Zap, CheckCircle } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000CodeEvaluator() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Brain className="inline w-8 h-8 mr-3" />
            c2000_code_evaluator
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            AI-powered analysis tool that evaluates code quality, analyzes runtime behavior, 
            compares implementations, and provides intelligent suggestions for improvement using Refact's internal LLM.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Analysis Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Semantic code analysis
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Runtime behavior evaluation
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Code comparison and diff analysis
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Performance optimization suggestions
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Best practices recommendations
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Analysis Types</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  UART output analysis
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Communication pattern detection
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Error pattern recognition
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Success/failure determination
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">UART Data Analysis</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_code_evaluator --uart-log spi_test.log --analyze-patterns
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Code Quality Check</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_code_evaluator --source spi_loopback.c --check-quality --suggest-improvements
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Compare Implementations</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_code_evaluator --compare spi_v1.c spi_v2.c --detailed-diff
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Performance Analysis</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_code_evaluator --profile runtime_data.json --optimize-suggestions
                </code>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6 mb-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Command Line Options</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <code className="text-green-300">--uart-log</code>
                  <span className="text-neutral-300">UART log file path</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--source</code>
                  <span className="text-neutral-300">Source code file</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--compare</code>
                  <span className="text-neutral-300">Compare two files</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--profile</code>
                  <span className="text-neutral-300">Runtime profile data</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--analyze-patterns</code>
                  <span className="text-neutral-300">Pattern analysis</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--suggest-improvements</code>
                  <span className="text-neutral-300">Generate suggestions</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>AI Analysis Capabilities</h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-purple-300">Pattern Recognition:</span>
                  <code className="text-neutral-300">Communication patterns</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Error Detection:</span>
                  <code className="text-neutral-300">Anomaly identification</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Code Quality:</span>
                  <code className="text-neutral-300">Best practices analysis</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Optimization:</span>
                  <code className="text-neutral-300">Performance suggestions</code>
                </div>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Analysis Report Example</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-orange-300 font-medium mb-2">SPI Loopback Analysis</h4>
                <pre className="text-sm text-neutral-300 bg-neutral-900 p-3 rounded overflow-x-auto">
{`{
  "analysis_type": "uart_pattern_analysis",
  "input_file": "spi_test.log",
  "status": "completed",
  "results": {
    "pattern_detection": {
      "loopback_test": {
        "status": "success",
        "confidence": 0.95,
        "pattern_matches": 156,
        "expected_patterns": 150
      },
      "error_patterns": {
        "detected": false,
        "error_count": 0
      }
    },
    "communication_analysis": {
      "baud_rate": 115200,
      "data_integrity": "excellent",
      "timing_consistency": "good",
      "throughput": "optimal"
    },
    "recommendations": [
      "Communication pattern shows successful loopback test",
      "No errors detected in SPI communication",
      "Consider adding error counters for production use",
      "Timing is within acceptable limits"
    ],
    "success_indicators": [
      "Consistent loopback pattern detected",
      "No communication errors",
      "Proper initialization sequence",
      "Expected data flow confirmed"
    ]
  },
  "analysis_time": "3.2s",
  "ai_model": "refact-internal-llm-v2.1"
}`}
                </pre>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Pattern Recognition</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Loopback test patterns
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Error message detection
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Status update patterns
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Data validation sequences
                </li>
              </ul>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>AI Integration</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Refact's internal LLM processing
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Context-aware analysis
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Continuous learning capabilities
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Domain-specific knowledge
                </li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
