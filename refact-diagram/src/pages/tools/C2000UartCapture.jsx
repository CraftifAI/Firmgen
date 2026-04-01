import { motion } from "framer-motion";
import { Monitor, Clock, FileText, Settings, Terminal, Activity } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000UartCapture() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Monitor className="inline w-8 h-8 mr-3" />
            c2000_uart_capture
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Captures and analyzes UART communication from C2000 devices, providing real-time monitoring, 
            data logging, and pattern analysis for debugging and verification purposes.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Capture Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Real-time data capture
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Configurable baud rates
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Data filtering and parsing
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Timestamp logging
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Export capabilities
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Supported Formats</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Raw binary data
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  ASCII text output
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Hex dump format
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  JSON structured data
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Basic Capture</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_uart_capture --port /dev/ttyUSB0 --baud 115200 --duration 30
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">With Filtering</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_uart_capture --port /dev/ttyUSB0 --baud 115200 --filter "SPI" --output spi_log.txt
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Continuous Monitoring</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_uart_capture --port /dev/ttyUSB0 --baud 115200 --continuous --format json
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Pattern Analysis</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_uart_capture --port /dev/ttyUSB0 --baud 115200 --analyze --pattern "loopback"
                </code>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6 mb-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Command Line Options</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <code className="text-green-300">--port</code>
                  <span className="text-neutral-300">UART port device</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--baud</code>
                  <span className="text-neutral-300">Baud rate</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--duration</code>
                  <span className="text-neutral-300">Capture duration (seconds)</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--output</code>
                  <span className="text-neutral-300">Output file path</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--format</code>
                  <span className="text-neutral-300">Output format</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--filter</code>
                  <span className="text-neutral-300">Data filter pattern</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>UART Configuration</h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-purple-300">Data Bits:</span>
                  <code className="text-neutral-300">8</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Stop Bits:</span>
                  <code className="text-neutral-300">1</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Parity:</span>
                  <code className="text-neutral-300">None</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Flow Control:</span>
                  <code className="text-neutral-300">None</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Buffer Size:</span>
                  <code className="text-neutral-300">64KB</code>
                </div>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Analysis Features</h3>
            <div className="space-y-4">
              <div className="grid md:grid-cols-2 gap-4">
                <div className="bg-neutral-800/50 rounded-lg p-4">
                  <h4 className="text-orange-300 font-medium mb-2">Pattern Detection</h4>
                  <ul className="text-sm text-neutral-300 space-y-1">
                    <li>• Loopback test patterns</li>
                    <li>• Error message detection</li>
                    <li>• Status update patterns</li>
                    <li>• Data validation sequences</li>
                  </ul>
                </div>
                <div className="bg-neutral-800/50 rounded-lg p-4">
                  <h4 className="text-orange-300 font-medium mb-2">Statistics</h4>
                  <ul className="text-sm text-neutral-300 space-y-1">
                    <li>• Data rate analysis</li>
                    <li>• Error count tracking</li>
                    <li>• Timing measurements</li>
                    <li>• Throughput calculations</li>
                  </ul>
                </div>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Output Formats</h3>
              <div className="space-y-3 text-sm">
                <div>
                  <h5 className="text-green-200 font-medium mb-1">Raw Binary:</h5>
                  <code className="text-neutral-300">Direct binary data capture</code>
                </div>
                <div>
                  <h5 className="text-green-200 font-medium mb-1">ASCII:</h5>
                  <code className="text-neutral-300">Human-readable text output</code>
                </div>
                <div>
                  <h5 className="text-green-200 font-medium mb-1">Hex Dump:</h5>
                  <code className="text-neutral-300">Hexadecimal representation</code>
                </div>
                <div>
                  <h5 className="text-green-200 font-medium mb-1">JSON:</h5>
                  <code className="text-neutral-300">Structured data with metadata</code>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Example Output</h3>
              <pre className="text-xs text-neutral-300 bg-neutral-900 p-3 rounded overflow-x-auto">
{`{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "data": "SPI Loopback Test: PASSED",
  "pattern": "loopback_success",
  "baud_rate": 115200,
  "duration": 30.5
}`}
              </pre>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
