import { motion } from "framer-motion";
import { Settings, Database, Cloud, FileText, RefreshCw, AlertTriangle } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function DynamicConfigurationSystem() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Settings className="inline w-8 h-8 mr-3" />
            Dynamic Configuration System
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            A sophisticated configuration management system that synchronizes runtime parameters, 
            handles fallback configurations, and ensures seamless operation across different environments.
          </p>

          <div className="grid md:grid-cols-3 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>
                <Cloud className="inline w-5 h-5 mr-2" />
                HTTP API Config
              </h3>
              <ul className="space-y-2 text-neutral-200 text-sm">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  <code className="bg-neutral-800 px-2 py-1 rounded text-xs">
                    http://localhost:8002/v1/c2000-config
                  </code>
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Real-time configuration updates
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  YAML to JSON conversion
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Comprehensive error handling
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  RESTful API integration
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>
                <Database className="inline w-5 h-5 mr-2" />
                Fallback Config
              </h3>
              <ul className="space-y-2 text-neutral-200 text-sm">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">•</span>
                  <code className="bg-neutral-800 px-2 py-1 rounded text-xs">
                    ~/.cache/refact/c2000_tools.yaml
                  </code>
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">•</span>
                  Offline operation support
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">•</span>
                  Automatic fallback mechanism
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">•</span>
                  Same structure as API config
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">•</span>
                  Local caching and persistence
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>
                <FileText className="inline w-5 h-5 mr-2" />
                Tool Execution
              </h3>
              <ul className="space-y-2 text-neutral-200 text-sm">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  CCS CLI Commands
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Hardware Operations
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  File Operations
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  AI Analysis Integration
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Cross-platform compatibility
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Configuration Flow</h3>
            <div className="space-y-4">
              <div className="flex items-center space-x-4">
                <div className="w-10 h-10 bg-blue-500 rounded-full flex items-center justify-center text-sm font-bold text-black">1</div>
                <div className="flex-1">
                  <h4 className="text-cyan-300 font-medium">API Request</h4>
                  <p className="text-sm text-neutral-300">System attempts to fetch configuration from HTTP API</p>
                </div>
              </div>
              <div className="flex items-center space-x-4">
                <div className="w-10 h-10 bg-purple-500 rounded-full flex items-center justify-center text-sm font-bold text-black">2</div>
                <div className="flex-1">
                  <h4 className="text-cyan-300 font-medium">Fallback Check</h4>
                  <p className="text-sm text-neutral-300">If API fails, automatically switches to local YAML config</p>
                </div>
              </div>
              <div className="flex items-center space-x-4">
                <div className="w-10 h-10 bg-green-500 rounded-full flex items-center justify-center text-sm font-bold text-black">3</div>
                <div className="flex-1">
                  <h4 className="text-cyan-300 font-medium">Validation</h4>
                  <p className="text-sm text-neutral-300">Configuration is validated and broadcast to all agents</p>
                </div>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-green-500/10 to-cyan-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>
                <RefreshCw className="inline w-5 h-5 mr-2" />
                Real-time Updates
              </h3>
              <ul className="space-y-2 text-neutral-200">
                <li>• Hot-reload configuration changes</li>
                <li>• Zero-downtime updates</li>
                <li>• Event-driven notifications</li>
                <li>• Automatic tool reconfiguration</li>
              </ul>
            </div>

            <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6">
              <h3 className={subtitle}>
                <AlertTriangle className="inline w-5 h-5 mr-2" />
                Error Handling
              </h3>
              <ul className="space-y-2 text-neutral-200">
                <li>• Graceful degradation</li>
                <li>• Comprehensive logging</li>
                <li>• Automatic retry mechanisms</li>
                <li>• User-friendly error messages</li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
