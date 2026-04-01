// File: src/components/RefactInteractiveFlow.jsx
import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown, ChevronUp } from "lucide-react";

const steps = [
  {
    title: "Refact Agent with Native C2000 Tools",
    desc: "Entry point managing all toolchain communication & orchestration.",
    substeps: [
      "Agent initializes runtime environment",
      "Discovers available Rust-based native tools",
      "Establishes session with C2000 SDK/CCS backend",
    ],
  },
  {
    title: "Natural Language Interface",
    desc: "Converts plain user commands into actionable intents.",
    substeps: [
      'Example: "Create SPI loopback project for F28P65x"',
      "Extracts keywords (target, example, config)",
      "Passes structured intent to AI parser",
    ],
  },
  {
    title: "AI Intent Parsing",
    desc: "Interprets high-level user goals into precise system operations.",
    substeps: [
      "Identifies which tool to use (project_create, flash, eval)",
      "Generates task dependency graph",
      "Validates configuration integrity before execution",
    ],
  },
  {
    title: "Dynamic Configuration System",
    desc: "Synchronizes runtime parameters & fallback YAML configs.",
    substeps: [
      "Fetches from remote JSON (caps.json)",
      "Overrides with CLI / user YAML inputs",
      "Broadcasts dynamic config updates to agents",
    ],
  },
  {
    title: "8 Native Rust Tools",
    desc: "Executes hardware-level actions using optimized Rust utilities.",
    substeps: [
      "c2000_project_create – create CCS project",
      "c2000_flash_tool – build & flash binary",
      "uart_log_collector – fetch UART debug logs",
      "ai_evaluator – analyze runtime metrics",
    ],
  },
  {
    title: "Complete Workflow Example",
    desc: "Runs end-to-end agentic flow using tool orchestration.",
    substeps: [
      "Builds SPI loopback project",
      "Flashes firmware to target",
      "Runs UART monitor & captures logs",
      "AI evaluator generates success/failure summary",
    ],
  },
  {
    title: "Final Report",
    desc: "Summarizes execution results and evaluation metrics.",
    substeps: [
      "Merges build + runtime + evaluation data",
      "Generates Markdown & JSON reports",
      "Uploads results to central dashboard",
    ],
  },
];

export default function RefactInteractiveFlow() {
  const [openIndex, setOpenIndex] = useState(null);

  return (
    <div className="bg-neutral-950 min-h-screen text-white flex flex-col items-center py-16 px-4">
      <motion.h1
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
        className="text-4xl font-bold mb-10 text-center text-cyan-400"
      >
        Refact Agentic Workflow — C2000 Toolchain
      </motion.h1>

      <div className="relative w-full max-w-5xl space-y-6">
        {steps.map((step, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: i * 0.2 }}
            className="relative bg-neutral-800 border border-neutral-700 hover:border-cyan-400 rounded-2xl shadow-lg p-6 transition-all duration-300"
          >
            <div
              className="flex justify-between items-center cursor-pointer"
              onClick={() => setOpenIndex(openIndex === i ? null : i)}
            >
              <h2 className="text-xl font-semibold text-cyan-300">
                {step.title}
              </h2>
              {openIndex === i ? (
                <ChevronUp className="text-cyan-300" />
              ) : (
                <ChevronDown className="text-cyan-300" />
              )}
            </div>

            <p className="text-neutral-300 mt-2">{step.desc}</p>

            <AnimatePresence>
              {openIndex === i && (
                <motion.ul
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.3 }}
                  className="mt-4 pl-5 space-y-2 text-neutral-200 list-disc"
                >
                  {step.substeps.map((sub, j) => (
                    <motion.li
                      key={j}
                      initial={{ opacity: 0, x: -10 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: j * 0.1 }}
                    >
                      {sub}
                    </motion.li>
                  ))}
                </motion.ul>
              )}
            </AnimatePresence>

            {i < steps.length - 1 && (
              <motion.div
                className="absolute left-1/2 bottom-[-40px] w-1 h-10 bg-cyan-500 rounded-full"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ delay: i * 0.2 + 0.4 }}
              />
            )}
          </motion.div>
        ))}
      </div>
    </div>
  );
}

