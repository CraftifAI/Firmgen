import { motion } from "framer-motion";
import { Play, Settings, Terminal, CheckCircle, AlertTriangle, Clock } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000Build() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Play className="inline w-8 h-8 mr-3" />
            c2000_build
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Compiles C2000 projects with optimal settings, handling multiple build configurations, 
            dependency resolution, and comprehensive error reporting.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Build Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Multi-configuration builds
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Parallel compilation
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Dependency tracking
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Incremental builds
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Cross-compilation support
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Build Configurations</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  CPU1_LAUNCHXL_RAM
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  CPU1_LAUNCHXL_FLASH
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  CPU1_CONTROLCARD_RAM
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  CPU1_CONTROLCARD_FLASH
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Basic Build</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_build --project /path/to/project --config CPU1_LAUNCHXL_RAM
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Clean Build</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_build --project /path/to/project --clean --config CPU1_LAUNCHXL_FLASH
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Parallel Build</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_build --project /path/to/project --jobs 4 --config CPU1_LAUNCHXL_RAM
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Verbose Output</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_build --project /path/to/project --verbose --config CPU1_LAUNCHXL_RAM
                </code>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6 mb-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Command Line Options</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <code className="text-green-300">--project</code>
                  <span className="text-neutral-300">Project directory path</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--config</code>
                  <span className="text-neutral-300">Build configuration</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--clean</code>
                  <span className="text-neutral-300">Clean before build</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--jobs</code>
                  <span className="text-neutral-300">Parallel job count</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--verbose</code>
                  <span className="text-neutral-300">Detailed output</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--output</code>
                  <span className="text-neutral-300">Output directory</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Build Output</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">project.out</code> - Executable binary
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">project.map</code> - Memory map file
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">build.log</code> - Build log
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">*.obj</code> - Object files
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">*.lst</code> - Assembly listings
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Build Configuration Details</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-orange-300 font-medium mb-2">CPU1_LAUNCHXL_RAM Configuration</h4>
                <div className="grid md:grid-cols-2 gap-4 text-sm">
                  <div>
                    <h5 className="text-orange-200 font-medium mb-1">Compiler Settings:</h5>
                    <ul className="text-neutral-300 space-y-1">
                      <li>• Optimization: -O2</li>
                      <li>• Debug info: -g</li>
                      <li>• Target: C28x</li>
                      <li>• FPU: Enabled</li>
                    </ul>
                  </div>
                  <div>
                    <h5 className="text-orange-200 font-medium mb-1">Memory Layout:</h5>
                    <ul className="text-neutral-300 space-y-1">
                      <li>• RAM: 0x080000</li>
                      <li>• FLASH: 0x080000</li>
                      <li>• Stack: 0x040000</li>
                      <li>• Heap: 0x020000</li>
                    </ul>
                  </div>
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
                  Detailed error reporting
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Warning suppression
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Dependency validation
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Build failure recovery
                </li>
              </ul>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Performance Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Incremental compilation
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Parallel processing
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Caching mechanisms
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Build time optimization
                </li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
