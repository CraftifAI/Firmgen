import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";

const blocks = [
  {
    id: 1,
    title: "Natural Language Interface",
    desc: 'User Input: "Create SPI loopback project for F28P65x"',
  },
  {
    id: 2,
    title: "AI Intent Parsing",
    points: [
      "Understands user request",
      "Identifies required tools",
      "Determines workflow sequence",
    ],
  },
  {
    id: 3,
    title: "Dynamic Configuration System",
    subblocks: [
      {
        title: "HTTP API Config",
        points: [
          "http://localhost:8002/v1/c2000-config",
          "Real-time updates",
          "YAML → JSON",
          "Error handling",
        ],
      },
      {
        title: "Fallback Config",
        points: [
          "/home/user/.cache/refact/c2000_tools.yaml",
          "Offline operation",
          "Automatic fallback",
          "Same structure",
        ],
      },
      {
        title: "Tool Execution",
        points: [
          "CCS CLI Commands",
          "Hardware Operations",
          "File Operations",
          "AI Analysis",
        ],
      },
    ],
  },
  {
    id: 4,
    title: "8 Native Rust Tools",
    subgroups: [
      {
        name: "Core Workflow Tools",
        items: ["c2000_project_create", "c2000_build", "c2000_flash", "c2000_uart_capture"],
      },
      {
        name: "Diagnostic & Support Tools",
        items: ["c2000_target_detect", "c2000_example_list", "c2000_config_validate"],
      },
      {
        name: "AI-Powered Analysis Tool",
        items: ["c2000_code_evaluator", "Semantic analysis", "Code comparison"],
      },
    ],
  },
  {
    id: 5,
    title: "Final Report",
    points: [
      "✅ Project created successfully",
      "✅ Built with CPU1_LAUNCHXL_RAM config",
      "✅ Flashed to F28P65x LaunchPad",
      "✅ UART output captured and analyzed",
      "✅ SPI loopback test completed",
    ],
  },
];

export default function RefactFlowAnimation() {
  const [activeIndex, setActiveIndex] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setActiveIndex((prev) => (prev + 1) % blocks.length);
    }, 4000);
    return () => clearInterval(timer);
  }, []);

  return (
    <div className="bg-neutral-950 min-h-screen text-white flex flex-col items-center py-16 px-6">
      <h1 className="text-3xl font-bold text-cyan-400 mb-12">
        Refact Agentic Flow — C2000 Toolchain
      </h1>

      {/* Prompt Bubble */}
      <AnimatePresence>
        {activeIndex === 0 && (
          <motion.div
            key="prompt"
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.6 }}
            className="bg-cyan-900/30 border border-cyan-500 rounded-2xl px-6 py-3 mb-10 shadow-lg text-cyan-200"
          >
            “Create SPI loopback project for F28P65x”
          </motion.div>
        )}
      </AnimatePresence>

      {blocks.map((block, i) => (
        <motion.div
          key={block.id}
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{
            opacity: 1,
            scale: activeIndex === i ? 1.05 : 1,
            boxShadow:
              activeIndex === i
                ? "0 0 25px #00ffff66"
                : "0 0 0px transparent",
          }}
          transition={{ duration: 0.6 }}
          className="bg-neutral-800 border border-cyan-500 rounded-2xl w-full max-w-4xl p-6 text-center mb-8 transition-all duration-300"
        >
          <h2 className="text-xl text-cyan-300 font-semibold mb-2">
            {block.title}
          </h2>

          {block.desc && <p className="text-neutral-200">{block.desc}</p>}

          {block.points && (
            <ul className="list-disc list-inside text-left text-neutral-300 mt-2">
              {block.points.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          )}

          {/* Sub-block groups */}
          {block.subblocks && activeIndex === i && (
            <div className="grid sm:grid-cols-3 gap-6 mt-4">
              {block.subblocks.map((sub) => (
                <motion.div
                  key={sub.title}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.5 }}
                  className="bg-neutral-900 border border-cyan-400 rounded-xl p-4 text-left text-sm"
                >
                  <h4 className="text-cyan-300 font-semibold mb-1">
                    {sub.title}
                  </h4>
                  <ul className="list-disc list-inside text-neutral-300">
                    {sub.points.map((p) => (
                      <li key={p}>{p}</li>
                    ))}
                  </ul>
                </motion.div>
              ))}
            </div>
          )}

          {/* Subgroups for Rust tools */}
          {block.subgroups && activeIndex === i && (
            <div className="grid sm:grid-cols-3 gap-6 mt-4 text-sm text-left">
              {block.subgroups.map((grp) => (
                <motion.div
                  key={grp.name}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.5 }}
                  className="bg-neutral-900 border border-cyan-400 rounded-xl p-4"
                >
                  <h4 className="text-cyan-300 font-semibold mb-2">
                    {grp.name}
                  </h4>
                  <ul className="list-disc list-inside text-neutral-300">
                    {grp.items.map((it) => (
                      <li key={it}>{it}</li>
                    ))}
                  </ul>
                </motion.div>
              ))}
            </div>
          )}
        </motion.div>
      ))}
    </div>
  );
}

