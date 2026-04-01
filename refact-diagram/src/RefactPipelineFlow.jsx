import React, { useCallback } from "react";
import ReactFlow, {
  MiniMap,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
} from "reactflow";
import "reactflow/dist/style.css";
import { motion } from "framer-motion";

const initialNodes = [
  {
    id: "1",
    data: { label: "Refact Agent\n(Native C2000 Tools)" },
    position: { x: 50, y: 150 },
    style: { background: "#111827", color: "#00ffff", borderRadius: "12px", padding: "10px", border: "1px solid #00ffff" },
  },
  {
    id: "2",
    data: { label: "Natural Language Interface" },
    position: { x: 320, y: 150 },
    style: { background: "#1e293b", color: "#00ffff", borderRadius: "12px", padding: "10px", border: "1px solid #00ffff" },
  },
  {
    id: "3",
    data: { label: "AI Intent Parsing" },
    position: { x: 610, y: 150 },
    style: { background: "#1e293b", color: "#00ffff", borderRadius: "12px", padding: "10px", border: "1px solid #00ffff" },
  },
  {
    id: "4",
    data: { label: "Dynamic Configuration System" },
    position: { x: 870, y: 150 },
    style: { background: "#1e293b", color: "#00ffff", borderRadius: "12px", padding: "10px", border: "1px solid #00ffff" },
  },
  {
    id: "5",
    data: { label: "8 Native Rust Tools" },
    position: { x: 1180, y: 150 },
    style: { background: "#1e293b", color: "#00ffff", borderRadius: "12px", padding: "10px", border: "1px solid #00ffff" },
  },
  {
    id: "6",
    data: { label: "Complete Workflow Example" },
    position: { x: 1500, y: 150 },
    style: { background: "#1e293b", color: "#00ffff", borderRadius: "12px", padding: "10px", border: "1px solid #00ffff" },
  },
  {
    id: "7",
    data: { label: "Final Report" },
    position: { x: 1830, y: 150 },
    style: { background: "#1e293b", color: "#00ffff", borderRadius: "12px", padding: "10px", border: "1px solid #00ffff" },
  },
];

const initialEdges = [
  { id: "e1-2", source: "1", target: "2", animated: true, style: { stroke: "#00ffff" } },
  { id: "e2-3", source: "2", target: "3", animated: true, style: { stroke: "#00ffff" } },
  { id: "e3-4", source: "3", target: "4", animated: true, style: { stroke: "#00ffff" } },
  { id: "e4-5", source: "4", target: "5", animated: true, style: { stroke: "#00ffff" } },
  { id: "e5-6", source: "5", target: "6", animated: true, style: { stroke: "#00ffff" } },
  { id: "e6-7", source: "6", target: "7", animated: true, style: { stroke: "#00ffff" } },
];

export default function RefactPipelineFlow() {
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const onConnect = useCallback(
    (params) => setEdges((eds) => addEdge({ ...params, animated: true, style: { stroke: "#00ffff" } }, eds)),
    [setEdges]
  );

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 1 }}
      className="w-screen h-screen bg-neutral-950"
    >
      <h1 className="text-center text-cyan-400 text-3xl font-bold py-6">
        Refact Agentic Workflow — C2000 Toolchain
      </h1>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        fitView
      >
        <MiniMap nodeColor="#00ffff" />
        <Controls />
        <Background variant="dots" gap={16} size={1} color="#0ff2" />
      </ReactFlow>
    </motion.div>
  );
}

