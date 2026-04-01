import { motion } from "framer-motion";
import { CheckCircle, AlertTriangle, Settings, Terminal, FileText, Shield } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000ConfigValidate() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Shield className="inline w-8 h-8 mr-3" />
            c2000_config_validate
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Validates project configurations, checks dependencies, performs compatibility analysis, 
            and ensures proper setup before build and deployment operations.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Validation Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Syntax checking
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Dependency validation
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Compatibility checks
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Error reporting
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Configuration optimization
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Validation Types</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Project configuration files
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Build settings
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Memory configurations
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Toolchain compatibility
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Basic Validation</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_config_validate --project /path/to/project
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Specific Configuration</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_config_validate --project /path/to/project --config CPU1_LAUNCHXL_RAM
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Dependency Check</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_config_validate --project /path/to/project --check-dependencies --verbose
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Fix Issues</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_config_validate --project /path/to/project --fix --backup
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
                  <span className="text-neutral-300">Specific build config</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--check-dependencies</code>
                  <span className="text-neutral-300">Validate dependencies</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--fix</code>
                  <span className="text-neutral-300">Auto-fix issues</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--backup</code>
                  <span className="text-neutral-300">Create backup before fix</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--verbose</code>
                  <span className="text-neutral-300">Detailed output</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Validation Checks</h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-purple-300">Syntax:</span>
                  <code className="text-neutral-300">YAML/JSON validation</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Dependencies:</span>
                  <code className="text-neutral-300">Library availability</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Memory:</span>
                  <code className="text-neutral-300">Layout validation</code>
                </div>
                <div className="flex justify-between">
                  <span className="text-purple-300">Toolchain:</span>
                  <code className="text-neutral-300">Version compatibility</code>
                </div>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Validation Report Example</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-orange-300 font-medium mb-2">Validation Results</h4>
                <pre className="text-sm text-neutral-300 bg-neutral-900 p-3 rounded overflow-x-auto">
{`{
  "project": "/path/to/project",
  "status": "valid",
  "checks": {
    "syntax": {
      "status": "pass",
      "errors": []
    },
    "dependencies": {
      "status": "pass",
      "missing": [],
      "version_mismatches": []
    },
    "memory": {
      "status": "pass",
      "total_flash": "512KB",
      "used_flash": "45KB",
      "total_ram": "100KB",
      "used_ram": "12KB"
    },
    "toolchain": {
      "status": "pass",
      "compiler_version": "20.2.0",
      "target_support": true
    }
  },
  "recommendations": [
    "Consider enabling optimization for production builds",
    "Memory usage is within safe limits"
  ],
  "validation_time": "1.2s"
}`}
                </pre>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Error Types</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-red-400 mr-2">✗</span>
                  Syntax errors in config files
                </li>
                <li className="flex items-start">
                  <span className="text-red-400 mr-2">✗</span>
                  Missing dependencies
                </li>
                <li className="flex items-start">
                  <span className="text-red-400 mr-2">✗</span>
                  Memory configuration conflicts
                </li>
                <li className="flex items-start">
                  <span className="text-red-400 mr-2">✗</span>
                  Toolchain version mismatches
                </li>
              </ul>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Auto-Fix Capabilities</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Correct syntax errors
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Update dependency paths
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Optimize memory settings
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  Update toolchain references
                </li>
              </ul>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
