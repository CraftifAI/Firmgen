import type { FirmwareGraph, GraphOrientation } from "../types";
import {
  DEFAULT_LAYER_GAP,
  DEFAULT_NODE_GAP,
  DEFAULT_NODE_WIDTH,
} from "../types";
import { applyLayoutToGraph, computeLayeredLayout } from "./layoutEngine";

function withLayoutMeta(
  graph: FirmwareGraph,
  orientation: GraphOrientation,
): FirmwareGraph {
  return {
    ...graph,
    layout: {
      ...graph.layout,
      orientation,
      node_width: DEFAULT_NODE_WIDTH,
      layer_gap: DEFAULT_LAYER_GAP,
      node_gap: DEFAULT_NODE_GAP,
    },
  };
}

export function nodeHasLayoutPosition(
  node: FirmwareGraph["nodes"][number],
): boolean {
  return (
    typeof node.visual?.x === "number" && typeof node.visual?.y === "number"
  );
}

function nodeHasPosition(node: FirmwareGraph["nodes"][number]): boolean {
  return nodeHasLayoutPosition(node);
}

export function graphHasLayoutCoordinates(graph: FirmwareGraph): boolean {
  return (
    graph.nodes.length > 0 && graph.nodes.every((n) => nodeHasLayoutPosition(n))
  );
}

/** Apply layered layout on the client and persist orientation in graph metadata. */
export function layoutGraph(
  graph: FirmwareGraph,
  orientation: GraphOrientation,
): FirmwareGraph {
  const { positions } = computeLayeredLayout(graph, orientation);
  const laidOut = applyLayoutToGraph(graph, positions);
  return withLayoutMeta(laidOut, orientation);
}

/** Layout only nodes that lack coordinates; keep existing x/y (user drag or prior layout). */
export function layoutGraphPreservingPositions(
  graph: FirmwareGraph,
  orientation: GraphOrientation,
): FirmwareGraph {
  const missing = graph.nodes.some((n) => !nodeHasPosition(n));
  if (!missing) {
    return withLayoutMeta(graph, orientation);
  }
  const { positions } = computeLayeredLayout(graph, orientation);
  const posMap = new Map(positions.map((p) => [p.node_id, p]));
  const merged: FirmwareGraph = {
    ...graph,
    nodes: graph.nodes.map((node) => {
      if (nodeHasPosition(node)) return node;
      const pos = posMap.get(node.id);
      if (!pos) return node;
      return {
        ...node,
        visual: {
          ...node.visual,
          x: pos.x,
          y: pos.y,
          layer: pos.layer,
        },
      };
    }),
  };
  return withLayoutMeta(merged, orientation);
}

/** Copy visual positions from a prior graph snapshot (e.g. before a patch response). */
export function mergeGraphVisualPositions(
  next: FirmwareGraph,
  prev: FirmwareGraph,
): FirmwareGraph {
  const posById = new Map(
    prev.nodes.map((n) => [n.id, n.visual] as const),
  );
  return {
    ...next,
    nodes: next.nodes.map((n) => {
      const visual = posById.get(n.id);
      if (!visual) return n;
      return { ...n, visual: { ...n.visual, ...visual } };
    }),
  };
}
