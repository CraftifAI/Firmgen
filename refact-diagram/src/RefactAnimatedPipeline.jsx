import React, { useEffect, useState } from "react";
import ReactFlow, {
  Background,
  MiniMap,
  Controls,
  useNodesState,
  useEdgesState,
} from "reactflow";
import "reactflow/dist/style.css";
import { Tooltip } from "react-tooltip";
import { motion } from "framer-motion";

const stages = [
  {
    id: "1",
    label: "Refact Agent\n(Native C2000 Tools)",
    details: [
      "Initializes runtime environment",
      "Discovers Rust-based native tools",
      "Connects with C2000 SDK/CCS",
    ],
  },
  {
    id: "2",
    label: "Natural Language Interface",
    details: [
      'Accepts commands like "Create SPI loopback for F28P65x"',
      "Extracts entities and context",
      "Passes structured request to parser",
    ],
  },
  {
    id: "3",
    label: "AI Intent Parsing",
    details: [
      "Understands user goals",
      "Determines required tools & sequence",
      "Prepares workflow graph",
    ],
  },
  {
    id: "4",
    label: "Dynamic Configuration System",
    details: [
      "Loads caps.json via HTTP",
      "Handles fallback YAML configs",
      "Syncs tool execution parameters",
    ],
  },
  {
    id: "5",
    label: "8 Native Rust Tools",
    details: [
      "c2000_project_create",
      "c2000_build / flash",
      "c2000_uart_capture",
      "c2000_code_evaluator",
    ],
  },
  {
    id: "6",
    label: "Complete Workflow Example",
    details: [
      "Discovery → Creation → Build → Flash → UART Capture",
      "AI analysis & report generation",
    ],
  },
  {
    id: "7",
    label: "Final Report",
    details: [
      "Aggregates results",
      "Highlights success/failure",
      "Suggests improvements",
    ],
  },
];

const initialEdges = stages.slice(0, -1).map((s, i) => ({
  id: `e${s.id}-${stages[i + 1].id}`,
  source: s.id,
  target: stages[i + 1].id,
  animated: true,
  style: { stroke: "#00ffff", strokeWidth: 2 },
}));

export default function RefactAnimatedPipeline() {
  const [nodes, setNodes] = useNodesState([]);
  const [edges] = useEdgesState(initialEdges);
  const [activeIndex, setActiveIndex] = useState(0);

  // Animate the reveal of nodes sequentially
  useEffect(() => {
    let i = 0;
    const interval = setInterval(() => {
      setActiveIndex(i);
      setNodes(
        stages.slice(0, i + 1).map((s, idx) => ({
          id: s.id,
          data: { label: s.label },
          position: { x: 180 * idx, y: 200 },
          style: {
            background:
              idx === i
                ? "linear-gradient(145deg,#00ffff33,#00ffff11)"
                : "#1e293b",
            color: "#00ffff",
            border: "1px solid #00ffff",
            borderRadius: 12,
            padding: 10,
            boxShadow:
              idx === i
                ? "0 0 12px #00ffffaa"
                : "0 0 5px rgba(0,0,0,0.3)",
          },
        }))
      );
      i++;
      if (i >= stages.length) i = 0; // loop
    }, 2500);

    return () => clearInterval(interval);
  }, []);

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 1 }}
      className="w-screen h-screen bg-neutral-950 text-white"
    >
      <h1 className="text-center text-cyan-400 text-3xl font-bold py-6">
        Refact Agentic Workflow — Animated Pipeline
      </h1>

      <ReactFlow nodes={nodes} edges={edges} fitView>
        <MiniMap nodeColor={() => "#00ffff"} />
        <Controls />
        <Background variant="dots" gap={16} size={1} color="#00ffff33" />
      </ReactFlow>

      <div className="absolute bottom-10 left-1/2 -translate-x-1/2 bg-neutral-800/70 p-6 rounded-xl border border-cyan-400 max-w-3xl">
        <h2 className="text-cyan-300 font-semibold text-xl mb-2 text-center">
          {stages[activeIndex]?.label.replace("\n", " ")}
        </h2>
        <ul className="list-disc list-inside text-neutral-200 text-sm">
          {stages[activeIndex]?.details.map((d, i) => (
            <li key={i}>{d}</li>
          ))}
        </ul>
      </div>

      <Tooltip id="tooltip" place="top" />
    </motion.div>
  );
}

