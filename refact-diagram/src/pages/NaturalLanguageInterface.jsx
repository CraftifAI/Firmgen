import { motion } from "framer-motion";
import { Code, Settings, Zap, Database, Cpu, FileText, CheckCircle } from "lucide-react";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg p-6";
const title = "text-2xl font-semibold text-cyan-300 mb-4";
const subtitle = "text-lg font-medium text-cyan-400 mb-3";

export default function NaturalLanguageInterface() {
  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6">
      <motion.div
        {...fadeIn}
        transition={{ duration: 0.6 }}
        className="max-w-4xl mx-auto"
      >
        <div className={card}>
          <h1 className={title}>
            <FileText className="inline w-8 h-8 mr-3" />
            Natural Language Interface
          </h1>
          
          <p className="text-neutral-300 text-lg mb-6">
            The entry point where users interact with the Refact Agent using plain English commands. 
            This interface converts natural language into structured intents that the AI can understand and process.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Example Commands</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  <code className="bg-neutral-800 px-2 py-1 rounded text-sm">
                    "Create SPI loopback project for F28P65x"
                  </code>
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  <code className="bg-neutral-800 px-2 py-1 rounded text-sm">
                    "Build and flash the project"
                  </code>
                </li>
                <li className="flex items-start">
                  <span className="text-cyan-400 mr-2">•</span>
                  <code className="bg-neutral-800 px-2 py-1 rounded text-sm">
                    "Monitor UART output for 30 seconds"
                  </code>
                </li>
              </ul>
            </div>

            <div className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4">
              <h3 className={subtitle}>Key Features</h3>
              <ul className="space-y-2 text-neutral-200">
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Context-aware command interpretation
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Multi-step workflow support
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Error handling and clarification
                </li>
                <li className="flex items-start">
                  <span className="text-green-400 mr-2">✓</span>
                  Real-time feedback and progress updates
                </li>
              </ul>
            </div>
          </div>

          <div className="bg-gradient-to-r from-cyan-500/10 to-blue-500/10 border border-cyan-500/30 rounded-xl p-6">
            <h3 className={subtitle}>Processing Flow</h3>
            <div className="space-y-3">
              <div className="flex items-center space-x-3">
                <div className="w-8 h-8 bg-cyan-500 rounded-full flex items-center justify-center text-sm font-bold text-black">1</div>
                <span className="text-neutral-200">User inputs natural language command</span>
              </div>
              <div className="flex items-center space-x-3">
                <div className="w-8 h-8 bg-cyan-500 rounded-full flex items-center justify-center text-sm font-bold text-black">2</div>
                <span className="text-neutral-200">System extracts keywords and intent</span>
              </div>
              <div className="flex items-center space-x-3">
                <div className="w-8 h-8 bg-cyan-500 rounded-full flex items-center justify-center text-sm font-bold text-black">3</div>
                <span className="text-neutral-200">Passes structured data to AI parser</span>
              </div>
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
