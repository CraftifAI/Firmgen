import { motion } from "framer-motion";
import { List, Search, FileText, Settings, Terminal, Folder } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000ExampleList() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <List className="inline w-8 h-8 mr-3" />
            c2000_example_list
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Searches and catalogs available C2000Ware examples, providing detailed information about 
            project templates, configurations, and usage examples for rapid project development.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Search Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  C2000Ware integration
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Advanced filtering capabilities
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Metadata extraction
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Path resolution
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Dependency analysis
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Example Categories</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Communication (SPI, UART, I2C)
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Control Systems (PWM, ADC)
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Motor Control (FOC, BLDC)
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Power Management
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">List All Examples</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_example_list --c2000ware-path /opt/ti/c2000ware
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Filter by Type</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_example_list --type spi --target F28P65x
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Search by Keyword</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_example_list --search "loopback" --json
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Detailed Information</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_example_list --example spi_loopback_f28p65x --verbose
                </code>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6 mb-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Command Line Options</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <code className="text-green-300">--c2000ware-path</code>
                  <span className="text-neutral-300">C2000Ware installation path</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--type</code>
                  <span className="text-neutral-300">Example type filter</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--target</code>
                  <span className="text-neutral-300">Target device filter</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--search</code>
                  <span className="text-neutral-300">Keyword search</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--example</code>
                  <span className="text-neutral-300">Specific example name</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--json</code>
                  <span className="text-neutral-300">JSON output format</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Search Filters</h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-purple-300">Type:</span>
                  <code className="text-neutral-300">spi, uart, i2c, pwm</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Target:</span>
                  <code className="text-neutral-300">F28P65x, F28P55x</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Complexity:</span>
                  <code className="text-neutral-300">basic, advanced</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Category:</span>
                  <code className="text-neutral-300">communication, control</code>
                </div>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Example Output</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-orange-300 font-medium mb-2">SPI Examples</h4>
                <pre className="text-sm text-neutral-300 bg-neutral-900 p-3 rounded overflow-x-auto">
{`[
  {
    "name": "spi_loopback_f28p65x",
    "type": "spi",
    "target": "F28P65x",
    "description": "SPI loopback test example",
    "path": "/opt/ti/c2000ware/examples/c28x/spi/spi_loopback_f28p65x",
    "files": [
      "spi_loopback.c",
      "spi_loopback.h",
      "spi_loopback.prj"
    ],
    "dependencies": [
      "driverlib",
      "common"
    ],
    "complexity": "basic",
    "category": "communication"
  }
]`}
                </pre>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Metadata Extraction</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Project file analysis
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Source code parsing
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Dependency detection
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Configuration extraction
                </li>
              </ul>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Integration Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Project template generation
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Dependency resolution
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Path validation
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Version compatibility
                </li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
