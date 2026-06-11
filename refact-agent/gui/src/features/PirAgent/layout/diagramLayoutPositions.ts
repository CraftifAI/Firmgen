import type { FirmwareGraph } from "../types";
import type { GraphDiagramView } from "./graphViewTypes";

export type DiagramNodePosition = { x: number; y: number };

export type DiagramLayoutPositions = Partial<
  Record<GraphDiagramView, Record<string, DiagramNodePosition>>
>;

export function extractDiagramPositions(
  graph: FirmwareGraph,
): Record<string, DiagramNodePosition> {
  const out: Record<string, DiagramNodePosition> = {};
  for (const node of graph.nodes) {
    const x = node.visual?.x;
    const y = node.visual?.y;
    if (typeof x === "number" && typeof y === "number") {
      out[node.id] = { x, y };
    }
  }
  return out;
}

export function applyDiagramPositions(
  graph: FirmwareGraph,
  positions: Record<string, DiagramNodePosition> | undefined,
): FirmwareGraph {
  if (!positions || Object.keys(positions).length === 0) return graph;
  return {
    ...graph,
    nodes: graph.nodes.map((node) => {
      if (!(node.id in positions)) return node;
      const pos = positions[node.id];
      return {
        ...node,
        visual: {
          ...node.visual,
          x: pos.x,
          y: pos.y,
        },
      };
    }),
  };
}
