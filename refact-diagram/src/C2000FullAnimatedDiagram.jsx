import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useNavigate } from "react-router-dom";
import { ExternalLink, Play } from "lucide-react";

const STEP_MS = 2500; // pacing

const card = "bg-neutral-800/95 border border-cyan-500 rounded-2xl shadow-lg";
const title = "text-xl font-semibold text-cyan-300";
const listUl = "list-disc list-inside text-neutral-200";

const fadeIn = {
  initial: { opacity: 0, y: 20 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -20 },
};

// === Diagram Blocks ===
const BLOCKS = [
  {
    title: "Natural Language Interface",
    desc: 'User Input: "Create SPI loopback project for F28P65x"',
    route: "/natural-language-interface",
    clickable: true,
  },
  {
    title: "AI Intent Parsing",
    items: [
      "Understands user request",
      "Identifies required tools",
      "Determines workflow sequence",
    ],
    route: "/ai-intent-parsing",
    clickable: true,
  },
  {
    title: "Dynamic Configuration System",
    subblocks: [
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
          "/home/shubham/.cache/refact/c2000_tools.yaml",
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
    ],
    route: "/dynamic-configuration-system",
    clickable: true,
  },
  {
    title: "8 Native Rust Tools",
    subblocks: [
      {
        title: "Core Workflow Tools",
        items: [
          "c2000_project_create",
          "c2000_build",
          "c2000_flash",
          "c2000_uart_capture",
        ],
      },
      {
        title: "Diagnostic & Support Tools",
        items: [
          "c2000_target_detect",
          "c2000_example_list",
          "c2000_config_validate",
        ],
      },
      {
        title: "AI-Powered Analysis Tool",
        items: [
          "c2000_code_evaluator",
          "Semantic analysis",
          "Code comparison",
        ],
      },
    ],
    route: "/native-rust-tools",
    clickable: true,
  },
  {
    title: "Complete Workflow Example",
    desc: '"Create SPI loopback for F28P65x"',
    route: "/complete-workflow-example",
    clickable: true,
  },
  {
    title: "STEP 1: DISCOVERY",
    items: [
      "c2000_example_list",
      "Searches C2000Ware",
      "Finds SPI examples",
      "Returns projectspec paths",
    ],
  },
  {
    title: "STEP 2: CREATION",
    items: [
      "c2000_project_create",
      "Creates CCS project",
      "Copies to workspace",
      "Configures for F28P65x",
    ],
  },
  {
    title: "STEP 3: BUILD & DEPLOYMENT",
    items: ["c2000_build", "Compiles project", "CPU1_LAUNCHXL_RAM"],
  },
  {
    title: "STEP 4: HARDWARE VERIFICATION",
    items: [
      "c2000_target_detect",
      "Detects F28P65x",
      "Verifies connection",
      "Lists debug probes",
    ],
  },
  {
    title: "STEP 5: PROGRAMMING",
    items: [
      "c2000_flash",
      "Programs firmware",
      "Verifies programming",
      "Resets device",
    ],
  },
  {
    title: "STEP 6: MONITORING",
    items: [
      "c2000_uart_capture",
      "Captures UART output",
      "30-second capture",
      "Saves to file",
    ],
  },
  {
    title: "STEP 7: AI ANALYSIS (c2000_code_evaluator)",
    items: [
      "Analyzes captured UART data",
      "Uses Refact's internal LLM",
      "Communication pattern analysis",
      "Suggests improvements",
      "Reports success/failure status",
    ],
  },
  {
    title: "Final Report",
    items: [
      "✅ Project created successfully",
      "✅ Built with CPU1_LAUNCHXL_RAM configuration",
      "✅ Flashed to F28P65x LaunchPad",
      "✅ UART output captured and analyzed",
      "✅ SPI loopback test completed",
    ],
    green: true,
  },
];

export default function C2000FullAnimatedDiagram() {
  const [active, setActive] = useState(0);
  const navigate = useNavigate();

  useEffect(() => {
    const id = setInterval(() => {
      setActive((i) => (i + 1) % BLOCKS.length);
    }, STEP_MS);
    return () => clearInterval(id);
  }, []);

  const handleBlockClick = (block) => {
    if (block.clickable && block.route) {
      navigate(block.route);
    }
  };

  return (
    <div className="bg-neutral-950 min-h-screen text-white py-10 px-6 flex flex-col items-center overflow-hidden">
      <h1 className="text-3xl font-bold text-cyan-400 mb-8 text-center">
        REFACT AGENTIC WORKFLOW — C2000 TOOLCHAIN
      </h1>

      {/* Journey Button */}
      <motion.button
        onClick={() => navigate('/prompt-journey')}
        className="mb-8 flex items-center space-x-3 px-6 py-3 bg-gradient-to-r from-purple-500 to-cyan-500 hover:from-purple-600 hover:to-cyan-600 rounded-xl transition-all duration-300 shadow-lg hover:shadow-xl"
        whileHover={{ scale: 1.05 }}
        whileTap={{ scale: 0.95 }}
      >
        <Play className="w-5 h-5" />
        <span className="font-semibold">Watch Prompt Journey</span>
        <ExternalLink className="w-4 h-4" />
      </motion.button>

      <div className="w-full max-w-5xl flex flex-col gap-6">
        {BLOCKS.map((b, i) => (
          <motion.div
            key={b.title}
            {...fadeIn}
            transition={{ duration: 0.6 }}
            className={`${card} p-5 transition-all duration-700 ${
              active === i
                ? "border-cyan-400 shadow-cyan-400/40 scale-[1.03]"
                : "border-neutral-700 scale-[0.98] opacity-70"
            } ${b.clickable ? "cursor-pointer hover:border-cyan-300 hover:shadow-cyan-300/20" : ""}`}
            onClick={() => handleBlockClick(b)}
          >
            <div className="flex items-center justify-center space-x-2">
              <h2 className={`${title} mb-2 text-center`}>{b.title}</h2>
              {b.clickable && (
                <ExternalLink className="w-5 h-5 text-cyan-400 opacity-60" />
              )}
            </div>
            {b.desc && (
              <p className="text-center text-neutral-300">{b.desc}</p>
            )}

            {/* Show sub-blocks or lists only if active */}
            <AnimatePresence>
              {active === i && (
                <>
                  {b.items && (
                    <motion.ul
                      {...fadeIn}
                      transition={{ duration: 0.5 }}
                      className={`mt-3 text-center space-y-1 ${
                        b.green ? "text-green-400" : "text-neutral-200"
                      }`}
                    >
                      {b.items.map((it) => (
                        <li
                          key={it}
                          className="list-none before:content-['▸'] before:text-cyan-400 before:mr-2"
                        >
                          {it}
                        </li>
                      ))}
                    </motion.ul>
                  )}

                  {b.subblocks && (
                    <motion.div
                      {...fadeIn}
                      transition={{ duration: 0.6 }}
                      className="grid sm:grid-cols-3 gap-6 mt-4"
                    >
                      {b.subblocks.map((s) => (
                        <div
                          key={s.title}
                          className="bg-neutral-900/80 border border-cyan-400 rounded-xl p-4"
                        >
                          <h4 className="text-cyan-300 font-semibold mb-2">
                            {s.title}
                          </h4>
                          <ul className={`${listUl} text-sm`}>
                            {s.items.map((it) => (
                              <li key={it}>{it}</li>
                            ))}
                          </ul>
                        </div>
                      ))}
                    </motion.div>
                  )}
                </>
              )}
            </AnimatePresence>
          </motion.div>
        ))}
      </div>
    </div>
  );
}

