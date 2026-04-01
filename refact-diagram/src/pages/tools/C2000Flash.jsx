import { motion } from "framer-motion";
import { Upload, CheckCircle, AlertTriangle, Settings, Terminal, Zap } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000Flash() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Upload className="inline w-8 h-8 mr-3" />
            c2000_flash
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Programs compiled firmware to C2000 target hardware, handles device detection, 
            memory verification, and provides comprehensive programming status reporting.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Programming Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Automatic device detection
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Memory verification
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Flash/EEPROM programming
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Device reset control
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Error recovery mechanisms
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Supported Interfaces</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  JTAG (IEEE 1149.1)
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  cJTAG (IEEE 1149.7)
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  SWD (Serial Wire Debug)
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  XDS110/XDS200 Debug Probes
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Basic Programming</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_flash --binary project.out --target F28P65x
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">With Verification</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_flash --binary project.out --target F28P65x --verify --reset
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Custom Memory Address</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_flash --binary project.out --address 0x080000 --target F28P65x
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Erase and Program</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_flash --binary project.out --target F28P65x --erase --verify
                </code>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6 mb-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Command Line Options</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <code className="text-green-300">--binary</code>
                  <span className="text-neutral-300">Binary file path</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--target</code>
                  <span className="text-neutral-300">Target device</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--address</code>
                  <span className="text-neutral-300">Memory address</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--verify</code>
                  <span className="text-neutral-300">Verify programming</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--erase</code>
                  <span className="text-neutral-300">Erase before program</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--reset</code>
                  <span className="text-neutral-300">Reset after program</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Memory Layout</h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-purple-300">FLASH Start:</span>
                  <code className="text-neutral-300">0x080000</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">FLASH Size:</span>
                  <code className="text-neutral-300">512KB</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">RAM Start:</span>
                  <code className="text-neutral-300">0x000000</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">RAM Size:</span>
                  <code className="text-neutral-300">100KB</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Boot Vector:</span>
                  <code className="text-neutral-300">0x080000</code>
                </div>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Programming Process</h3>
            <div className="space-y-4">
              <div className="space-y-3">
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 bg-orange-500 rounded-full flex items-center justify-center text-sm font-bold text-black">1</div>
                  <span className="text-neutral-200">Device detection and connection validation</span>
                </div>
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 bg-orange-500 rounded-full flex items-center justify-center text-sm font-bold text-black">2</div>
                  <span className="text-neutral-200">Memory erase (if requested)</span>
                </div>
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 bg-orange-500 rounded-full flex items-center justify-center text-sm font-bold text-black">3</div>
                  <span className="text-neutral-200">Binary file loading and programming</span>
                </div>
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 bg-orange-500 rounded-full flex items-center justify-center text-sm font-bold text-black">4</div>
                  <span className="text-neutral-200">Memory verification (if requested)</span>
                </div>
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 bg-orange-500 rounded-full flex items-center justify-center text-sm font-bold text-black">5</div>
                  <span className="text-neutral-200">Device reset and execution start</span>
                </div>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Error Handling</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Connection failure recovery
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Programming error detection
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Verification failure handling
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Device reset recovery
                </li>
              </ul>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Status Reporting</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Programming progress
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Verification results
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Timing information
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Success/failure status
                </li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
