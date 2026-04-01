import { motion } from "framer-motion";
import { Brain, Target, Workflow, CheckCircle } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function AIIntentParsing() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-4xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <Brain className="inline w-8 h-8 mr-3" />
            AI Intent Parsing
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            The AI-powered component that interprets high-level user goals and converts them into 
            precise system operations. This is where natural language gets transformed into actionable tasks.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Core Capabilities</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Understands user request context
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Identifies required tools and dependencies
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Determines optimal workflow sequence
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  Validates configuration integrity
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Processing Steps</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Intent classification and extraction
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Tool selection and parameter mapping
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Dependency graph generation
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Execution plan optimization
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-purple-500/10 to-cyan-500/10 border border-purple-500/30 rounded-xl p-6 mb-6">
            <h3 className={subtitle}>Example: "Create SPI loopback project for F28P65x"</h3>
            <div className="space-y-4">
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-cyan-300 font-medium mb-2">Extracted Intent:</h4>
                <ul className="text-neutral-200 space-y-1">
                  <li>• Action: Create project</li>
                  <li>• Type: SPI loopback</li>
                  <li>• Target: F28P65x microcontroller</li>
                </ul>
              </div>
              <div className="bg-neutral-800/50 rounded-lg p-4">
                <h4 className="text-cyan-300 font-medium mb-2">Required Tools:</h4>
                <ul className="text-neutral-200 space-y-1">
                  <li>• c2000_example_list (discovery)</li>
                  <li>• c2000_project_create (creation)</li>
                  <li>• c2000_build (compilation)</li>
                  <li>• c2000_flash (deployment)</li>
                </ul>
              </div>
            </div>
          </div>

          <div className="bg-gradient-to-r from-green-500/10 to-blue-500/10 border border-green-500/30 rounded-xl p-6">
            <h3 className={subtitle}>Advanced Features</h3>
            <div className="grid md:grid-cols-3 gap-4">
              <div className="text-center">
                <Target className="w-8 h-8 text-green-400 mx-auto mb-2" />
                <h4 className="text-cyan-300 font-medium mb-1">Smart Targeting</h4>
                <p className="text-sm text-neutral-300">Automatically detects hardware capabilities and constraints</p>
              </div>
              <div className="text-center">
                <Workflow className="w-8 h-8 text-blue-400 mx-auto mb-2" />
                <h4 className="text-cyan-300 font-medium mb-1">Workflow Optimization</h4>
                <p className="text-sm text-neutral-300">Generates efficient execution sequences</p>
              </div>
              <div className="text-center">
                <CheckCircle className="w-8 h-8 text-purple-400 mx-auto mb-2" />
                <h4 className="text-cyan-300 font-medium mb-1">Validation</h4>
                <p className="text-sm text-neutral-300">Pre-execution error detection and prevention</p>
              </div>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
