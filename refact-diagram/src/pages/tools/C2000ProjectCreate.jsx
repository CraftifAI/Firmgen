import { motion } from "framer-motion";
import { FileText, Play, Upload, Monitor, Code, Settings, Terminal, CheckCircle } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function C2000ProjectCreate() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-5xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <FileText className="inline w-8 h-8 mr-3" />
            c2000_project_create
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            Creates new Code Composer Studio (CCS) projects from templates, configuring them for specific 
            C2000 microcontrollers and setting up the complete project structure.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Core Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Template-based project creation
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Automatic target configuration
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Workspace structure setup
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Dependency management
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Build configuration generation
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Supported Targets</h3>
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
                  Custom hardware configurations
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Command Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Basic Project Creation</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_project_create --template spi_loopback --target F28P65x --name "my_spi_project"
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">Advanced Configuration</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_project_create --template uart_echo --target F28P55x --config custom --workspace /path/to/workspace
                </code>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-blue-300 font-medium mb-2">From C2000Ware Example</h4>
                <code className="bg-neutral-900 px-3 py-2 rounded text-green-300 block">
                  c2000_project_create --example-path /path/to/c2000ware/examples --project spi_loopback_f28p65x
                </code>
              </div>
            </div>
          </div>

          <div className="grid md:grid-cols-2 gap-6 mb-6">
            <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Command Line Options</h3>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between">
                  <code className="text-green-300">--template</code>
                  <span className="text-neutral-300">Project template name</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--target</code>
                  <span className="text-neutral-300">Target microcontroller</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--name</code>
                  <span className="text-neutral-300">Project name</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--workspace</code>
                  <span className="text-neutral-300">Workspace directory</span>
                </div>
                <div className="flex justify-between">
                  <code className="text-green-300">--config</code>
                  <span className="text-neutral-300">Build configuration</span>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-500/30 rounded-xl p-6">
              <h3 className={subtitle}>Generated Files</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">project.c</code> - Main application file
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">project.h</code> - Header definitions
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">project.prj</code> - CCS project file
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">linker.cmd</code> - Memory map
                </li>
                <li className="flex items-start">
                  <span className="text-purple-400 mr-2">•</span>
                  <code className="text-sm">build_configs/</code> - Build settings
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-orange-500/10 to-red-500/10 border border-orange-500/30 rounded-xl p-6">
            <h3 className={subtitle}>Configuration Examples</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-orange-300 font-medium mb-2">SPI Loopback Configuration</h4>
                <pre className="text-sm text-neutral-300 bg-neutral-900 p-3 rounded overflow-x-auto">
{`{
  "target": "F28P65x",
  "template": "spi_loopback",
  "configurations": {
    "CPU1_LAUNCHXL_RAM": {
      "memory": "RAM",
      "optimization": "O2",
      "debug": true
    },
    "CPU1_LAUNCHXL_FLASH": {
      "memory": "FLASH",
      "optimization": "Os",
      "debug": false
    }
  }
}`}
                </pre>
              </div>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
