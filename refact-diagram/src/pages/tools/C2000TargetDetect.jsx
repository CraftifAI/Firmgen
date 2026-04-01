import { motion } from "framer-motion";
import { Search, CheckCircle, AlertTriangle, Settings, Terminal, Cpu } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000TargetDetect() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Search className="inline w-8 h-8 mr-3" />
            c2000_target_detect
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Automatically detects and identifies connected C2000 hardware, validates debug probe connections, 
            and provides comprehensive device information for programming and debugging operations.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Detection Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Automatic hardware enumeration
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Debug probe identification
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Connection validation
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Device capability reporting
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Status monitoring
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Supported Hardware</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  F28P65x LaunchPad
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  F28P55x ControlCARD
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  F28P33x LaunchPad
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Custom evaluation boards
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Basic Detection</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_target_detect
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Verbose Output</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_target_detect --verbose --json
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Specific Probe</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_target_detect --probe XDS110 --target F28P65x
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Connection Test</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_target_detect --test-connection --timeout 10
                </code>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6 mb-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Command Line Options</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <code className="text-green-300">--verbose</code>
                  <span className="text-neutral-300">Detailed output</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--json</code>
                  <span className="text-neutral-300">JSON output format</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--probe</code>
                  <span className="text-neutral-300">Specific debug probe</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--target</code>
                  <span className="text-neutral-300">Target device type</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--test-connection</code>
                  <span className="text-neutral-300">Test communication</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--timeout</code>
                  <span className="text-neutral-300">Detection timeout (seconds)</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Detection Process</h3>
              <div className="space-y-2 text-sm">
                <div className="flex items-center space-x-2">
                  <div className="w-4 h-4 bg-purple-500 rounded-full"></div>
                  <span className="text-neutral-300">Scan USB ports</span>
                </div>
                <div className="flex items-center space-x-2">
                  <div className="w-4 h-4 bg-purple-500 rounded-full"></div>
                  <span className="text-neutral-300">Identify debug probes</span>
                </div>
                <div className="flex items-center space-x-2">
                  <div className="w-4 h-4 bg-purple-500 rounded-full"></div>
                  <span className="text-neutral-300">Connect to targets</span>
                </div>
                <div className="flex items-center space-x-2">
                  <div className="w-4 h-4 bg-purple-500 rounded-full"></div>
                  <span className="text-neutral-300">Read device IDs</span>
                </div>
                <div className="flex items-center space-x-2">
                  <div className="w-4 h-4 bg-purple-500 rounded-full"></div>
                  <span className="text-neutral-300">Validate capabilities</span>
                </div>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Example Output</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-orange-300 font-medium mb-2">JSON Output</h4>
                <pre className="text-sm text-neutral-300 bg-neutral-900 p-3 rounded overflow-x-auto">
{`{
  "probes": [
    {
      "id": "XDS110-USB",
      "type": "XDS110",
      "version": "2.0.0.15",
      "status": "connected",
      "targets": [
        {
          "id": "F28P65x",
          "name": "F28P65x LaunchPad",
          "status": "halted",
          "memory": {
            "flash": "512KB",
            "ram": "100KB"
          },
          "capabilities": [
            "flash_programming",
            "debugging",
            "trace"
          ]
        }
      ]
    }
  ],
  "summary": {
    "total_probes": 1,
    "total_targets": 1,
    "detection_time": "2.3s"
  }
}`}
                </pre>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Error Handling</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Connection timeout handling
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Probe communication errors
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Target identification failures
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Capability validation errors
                </li>
              </ul>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Use Cases</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Pre-programming validation
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Hardware troubleshooting
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Multi-target environments
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Automated testing setups
                </li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
