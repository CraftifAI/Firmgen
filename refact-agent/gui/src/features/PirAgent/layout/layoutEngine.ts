import type { FirmwareGraph, FirmwareNode, GraphOrientation, LayoutPosition } from "../types";
import { hldNodeHeight } from "./hldComponent";
import type { HldComponent } from "./hldComponent";
import { lddNodeHeight } from "./lddUml";
import type { LddUmlClass } from "./lddUml";
import {
  DEFAULT_LAYER_GAP,
  DEFAULT_NODE_GAP,
  DEFAULT_NODE_HEIGHT,
  DEFAULT_NODE_WIDTH,
} from "../types";

const NODE_W = DEFAULT_NODE_WIDTH;
const NODE_H = DEFAULT_NODE_HEIGHT;
const LAYER_GAP = DEFAULT_LAYER_GAP;
const NODE_GAP = DEFAULT_NODE_GAP;

function layoutNodeHeight(node: FirmwareNode, fallback: number): number {
  const hldRaw = node.properties?.hld_component;
  if (hldRaw && typeof hldRaw === "object") {
    const hld = hldRaw as HldComponent;
    if (hld.componentName && Array.isArray(hld.methods)) {
      return hldNodeHeight(hld);
    }
  }
  const lddRaw = node.properties?.ldd_uml;
  if (lddRaw && typeof lddRaw === "object") {
    const uml = lddRaw as LddUmlClass;
    if (uml.className && Array.isArray(uml.attributes) && Array.isArray(uml.methods)) {
      return lddNodeHeight(uml);
    }
  }
  return fallback;
}

export type ClientLayoutResult = {
  positions: LayoutPosition[];
  hasCycles: boolean;
};

/**
 * Client-side layered DAG layout (mirrors Rust engine for offline preview).
 */
export function computeLayeredLayout(
  graph: FirmwareGraph,
  orientation: GraphOrientation = "horizontal",
): ClientLayoutResult {
  const nodeW = graph.layout?.node_width ?? NODE_W;
  const defaultNodeH = NODE_H;
  const layerGap = graph.layout?.layer_gap ?? LAYER_GAP;
  const nodeGap = graph.layout?.node_gap ?? NODE_GAP;
  const nodeById = new Map(graph.nodes.map((n) => [n.id, n]));

  const portToNode = new Map<string, string>();
  for (const node of graph.nodes) {
    for (const port of node.ports) {
      portToNode.set(port.id, node.id);
    }
  }

  const inDegree = new Map<string, number>();
  const children = new Map<string, string[]>();
  for (const node of graph.nodes) {
    inDegree.set(node.id, 0);
  }

  for (const [srcPort, dstPort] of graph.connections) {
    const src = portToNode.get(srcPort);
    const dst = portToNode.get(dstPort);
    if (!src || !dst || src === dst) continue;
    children.set(src, [...(children.get(src) ?? []), dst]);
    inDegree.set(dst, (inDegree.get(dst) ?? 0) + 1);
  }

  const layer = new Map<string, number>();
  const placed = new Set<string>();
  const queue: string[] = [];
  for (const [id, deg] of inDegree) {
    if (deg === 0) {
      queue.push(id);
      layer.set(id, 0);
    }
  }

  let processed = 0;
  while (queue.length > 0) {
    const nodeId = queue.shift();
    if (nodeId === undefined) break;
    processed += 1;
    placed.add(nodeId);
    const currentLayer = layer.get(nodeId) ?? 0;
    for (const child of children.get(nodeId) ?? []) {
      layer.set(child, Math.max(layer.get(child) ?? 0, currentLayer + 1));
      const nextDeg = (inDegree.get(child) ?? 1) - 1;
      inDegree.set(child, nextDeg);
      if (nextDeg === 0) queue.push(child);
    }
  }

  const hasCycles = processed < graph.nodes.length;
  if (hasCycles) {
    const perRow = 4;
    let extra = 0;
    for (const node of graph.nodes) {
      if (placed.has(node.id)) continue;
      layer.set(node.id, Math.floor(extra / perRow));
      extra += 1;
    }
  }

  const diagramView = graph.runtime_metadata?.overlays?.diagram_view;
  if (diagramView === "hld") {
    const tierOrder: Record<string, number> = {
      entry: 0,
      control: 1,
      io: 2,
      connectivity: 3,
      storage: 4,
    };
    for (const node of graph.nodes) {
      const hldRaw = node.properties?.hld_component;
      if (!hldRaw || typeof hldRaw !== "object") continue;
      const tier = (hldRaw as HldComponent).tier;
      const minLayer = tierOrder[tier];
      layer.set(node.id, Math.max(layer.get(node.id) ?? 0, minLayer));
    }
  }

  const maxLayer = Math.max(0, ...layer.values());
  const layers: string[][] = Array.from({ length: maxLayer + 1 }, () => []);
  for (const node of graph.nodes) {
    layers[layer.get(node.id) ?? 0].push(node.id);
  }

  const positions: LayoutPosition[] = [];
  if (orientation === "vertical") {
    let layerY = 0;
    layers.forEach((nodesInLayer, layerIdx) => {
      const count = nodesInLayer.length;
      const heights = nodesInLayer.map((id) => {
        const n = nodeById.get(id);
        return layoutNodeHeight(n ?? { id, node_type: "", ports: [] }, defaultNodeH);
      });
      const maxH = Math.max(...heights, defaultNodeH);
      const rowWidth = count * nodeW + Math.max(0, count - 1) * nodeGap;
      nodesInLayer.forEach((nodeId, i) => {
        const x = i * (nodeW + nodeGap) - rowWidth / 2 + nodeW / 2;
        positions.push({ node_id: nodeId, x, y: layerY, layer: layerIdx });
      });
      layerY += maxH + layerGap;
    });
  } else {
    layers.forEach((nodesInLayer, layerIdx) => {
      const heights = nodesInLayer.map((id) => {
        const n = nodeById.get(id);
        return layoutNodeHeight(n ?? { id, node_type: "", ports: [] }, defaultNodeH);
      });
      const totalH =
        heights.reduce((sum, h) => sum + h, 0) +
        Math.max(0, nodesInLayer.length - 1) * nodeGap;
      let yCursor = -totalH / 2;
      nodesInLayer.forEach((nodeId, i) => {
        const nodeH = heights[i] ?? defaultNodeH;
        const x = layerIdx * (nodeW + layerGap);
        const y = yCursor;
        yCursor += nodeH + nodeGap;
        positions.push({ node_id: nodeId, x, y, layer: layerIdx });
      });
    });
  }

  return { positions, hasCycles };
}

export function applyLayoutToGraph(
  graph: FirmwareGraph,
  positions: LayoutPosition[],
): FirmwareGraph {
  const posMap = new Map(positions.map((p) => [p.node_id, p]));
  return {
    ...graph,
    nodes: graph.nodes.map((node) => {
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
}
