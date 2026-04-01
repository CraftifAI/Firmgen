import { motion } from "framer-motion";

const fadeUp = (delay = 0) => ({
  hidden: { opacity: 0, y: -30 },
  visible: {
    opacity: 1,
    y: 0,
    transition: { delay, duration: 0.6, ease: "easeOut" },
  },
});

export default function RefactStructuredDiagram() {
  return (
    <div className="bg-neutral-950 text-white min-h-screen py-16 px-6 flex flex-col items-center overflow-x-hidden">
      <motion.h1
        variants={fadeUp(0.1)}
        initial="hidden"
        animate="visible"
        className="text-3xl font-bold text-cyan-400 mb-12"
      >
        REFACT AGENT WITH NATIVE C2000 TOOLS
      </motion.h1>

      {/* Block 1 */}
      <motion.div
        variants={fadeUp(0.2)}
        initial="hidden"
        animate="visible"
        className="bg-neutral-800 border border-cyan-500 rounded-2xl w-full max-w-3xl p-6 text-center mb-10"
      >
        <h2 className="text-xl text-cyan-300 font-semibold mb-2">
          Natural Language Interface
        </h2>
        <p>User Input: "Create SPI loopback project for F28P65x"</p>
      </motion.div>

      {/* Block 2 */}
      <motion.div
        variants={fadeUp(0.3)}
        initial="hidden"
        animate="visible"
        className="bg-neutral-800 border border-cyan-500 rounded-2xl w-full max-w-3xl p-6 text-center mb-10"
      >
        <h2 className="text-xl text-cyan-300 font-semibold mb-2">
          AI Intent Parsing
        </h2>
        <ul className="text-left list-disc list-inside">
          <li>Understands user request</li>
          <li>Identifies required tools</li>
          <li>Determines workflow sequence</li>
        </ul>
      </motion.div>

      {/* Dynamic Config System - 3 parallel blocks */}
      <motion.div
        variants={fadeUp(0.4)}
        initial="hidden"
        animate="visible"
        className="bg-neutral-800 border border-cyan-500 rounded-2xl w-full max-w-3xl p-6 text-center mb-4"
      >
        <h2 className="text-xl text-cyan-300 font-semibold mb-2">
          Dynamic Configuration System
        </h2>
      </motion.div>
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 max-w-5xl mb-10">
        {[
          {
            title: "HTTP API Config",
            items: [
              "http://localhost:8002/v1/c2000-config",
              "Real-time updates",
              "YAML → JSON",
              "Error handling",
            ],
          },
          {
            title: "Fallback Config",
            items: [
              "/home/user/.cache/refact/c2000_tools.yaml",
              "Offline operation",
              "Automatic fallback",
              "Same structure",
            ],
          },
          {
            title: "Tool Execution",
            items: [
              "CCS CLI Commands",
              "Hardware Operations",
              "File Operations",
              "AI Analysis",
            ],
          },
        ].map((b, i) => (
          <motion.div
            key={b.title}
            variants={fadeUp(0.5 + i * 0.1)}
            initial="hidden"
            animate="visible"
            className="bg-neutral-800 border border-cyan-500 rounded-2xl p-5"
          >
            <h3 className="text-cyan-300 font-semibold mb-2">{b.title}</h3>
            <ul className="text-sm list-disc list-inside text-neutral-200 text-left">
              {b.items.map((it) => (
                <li key={it}>{it}</li>
              ))}
            </ul>
          </motion.div>
        ))}
      </div>

      {/* Native Rust Tools */}
      <motion.div
        variants={fadeUp(0.9)}
        initial="hidden"
        animate="visible"
        className="bg-neutral-800 border border-cyan-500 rounded-2xl w-full max-w-3xl p-6 text-center mb-10"
      >
        <h2 className="text-xl text-cyan-300 font-semibold mb-4">
          8 Native Rust Tools
        </h2>
        <div className="grid sm:grid-cols-3 gap-6 text-left text-sm">
          <div>
            <h4 className="font-semibold mb-1">Core Workflow Tools</h4>
            <ul className="list-disc list-inside">
              <li>c2000_project_create</li>
              <li>c2000_build</li>
              <li>c2000_flash</li>
              <li>c2000_uart_capture</li>
            </ul>
          </div>
          <div>
            <h4 className="font-semibold mb-1">Diagnostic & Support Tools</h4>
            <ul className="list-disc list-inside">
              <li>c2000_target_detect</li>
              <li>c2000_example_list</li>
              <li>c2000_config_validate</li>
            </ul>
          </div>
          <div>
            <h4 className="font-semibold mb-1">AI-Powered Analysis Tool</h4>
            <ul className="list-disc list-inside">
              <li>c2000_code_evaluator</li>
              <li>Semantic analysis</li>
              <li>Code comparison</li>
            </ul>
          </div>
        </div>
      </motion.div>

      {/* Final Report */}
      <motion.div
        variants={fadeUp(1.2)}
        initial="hidden"
        animate="visible"
        className="bg-neutral-800 border border-cyan-500 rounded-2xl w-full max-w-3xl p-6 text-center"
      >
        <h2 className="text-xl text-cyan-300 font-semibold mb-3">
          Final Report
        </h2>
        <ul className="text-left list-disc list-inside text-green-400">
          <li>✅ Project created successfully</li>
          <li>✅ Built with CPU1_LAUNCHXL_RAM config</li>
          <li>✅ Flashed to F28P65x LaunchPad</li>
          <li>✅ UART output captured and analyzed</li>
          <li>✅ SPI loopback test completed</li>
        </ul>
      </motion.div>
    </div>
  );
}

