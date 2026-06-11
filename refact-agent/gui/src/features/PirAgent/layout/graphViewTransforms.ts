import type {
  FirmwareGraph,
  FirmwareNode,
  GraphOrientation,
  NodeTypeDef,
} from "../types";
import type { PirEdge, PirNode } from "../pirTypes";
import { computeLayeredLayout } from "./layoutEngine";
import type { GraphDiagramView } from "./graphViewTypes";
import {
  buildHldComponent,
  buildHldEdgeLabelMap,
  hldAssociationPortId,
  hldLayoutNodeWidth,
} from "./hldComponent";
import {
  buildLddEdgeLabelMap,
  lddAssociationPortId,
  lddLayoutNodeWidth,
} from "./lddUml";

function cloneNode(node: FirmwareNode, patch: Partial<FirmwareNode>): FirmwareNode {
  return { ...node, ...patch, visual: { ...node.visual, ...patch.visual } };
}

function pirById(pirNodes: PirNode[]): Map<string, PirNode> {
  return new Map(pirNodes.map((n) => [n.id, n]));
}

function portId(nodeId: string, direction: "in" | "out"): string {
  return `${nodeId}::${direction}`;
}

const HLD_TIER_ORDER: Record<string, number> = {
  entry: 0,
  control: 1,
  io: 2,
  connectivity: 3,
  storage: 4,
};

const PERIPHERAL_CATEGORIES = new Set([
  "gpio",
  "sensors",
  "analog",
  "communication",
  "media",
]);

function isPeripheralNodeType(nodeType: string, registry: NodeTypeDef[]): boolean {
  const category = registry.find((def) => def.node_type === nodeType)?.category;
  if (category) {
    return PERIPHERAL_CATEGORIES.has(category);
  }
  return [
    "gpio_input",
    "gpio_output",
    "sensor_input",
    "pwm_output",
    "adc_reader",
    "i2c_device",
    "spi_device",
    "uart_device",
    "display_output",
    "camera_capture",
  ].includes(nodeType);
}

function buildHldGraph(
  graph: FirmwareGraph,
  pirNodes: PirNode[],
  pirEdges: PirEdge[],
): FirmwareGraph {
  const pirMap = pirById(pirNodes);
  const portToNode = new Map<string, string>();
  for (const node of graph.nodes) {
    for (const port of node.ports) {
      portToNode.set(port.id, node.id);
    }
  }

  const nodeById = new Map(graph.nodes.map((n) => [n.id, n]));
  const edgeKeys = new Set<string>();
  const connections: [string, string][] = [];

  for (const [srcPort, dstPort] of graph.connections) {
    const srcNode = portToNode.get(srcPort);
    const dstNode = portToNode.get(dstPort);
    if (!srcNode || !dstNode || srcNode === dstNode) continue;
    const key = `${srcNode}->${dstNode}`;
    if (edgeKeys.has(key)) continue;
    edgeKeys.add(key);
    connections.push([
      hldAssociationPortId(srcNode, "out"),
      hldAssociationPortId(dstNode, "in"),
    ]);
  }

  const nodes = graph.nodes.map((node) => {
    const pir = pirMap.get(node.id);
    const hld = buildHldComponent(node, pir);
    return cloneNode(node, {
      label: hld.componentName,
      description: pir?.ai_summary ?? node.description,
      ports: [
        {
          id: hldAssociationPortId(node.id, "in"),
          name: "assoc_in",
          direction: "input",
          datatype: "association",
        },
        {
          id: hldAssociationPortId(node.id, "out"),
          name: "assoc_out",
          direction: "output",
          datatype: "association",
        },
      ],
      properties: {
        ...node.properties,
        hld_component: hld,
      },
      visual: {
        ...node.visual,
        layer: HLD_TIER_ORDER[hld.tier],
      },
    });
  });

  const hldEdgeLabels = buildHldEdgeLabelMap(
    graph.connections,
    portToNode,
    nodeById,
    pirEdges,
  );

  const remappedLabels: Record<string, string> = {};
  for (const [srcPort, dstPort] of graph.connections) {
    const srcNode = portToNode.get(srcPort);
    const dstNode = portToNode.get(dstPort);
    if (!srcNode || !dstNode) continue;
    const oldKey = `${srcPort}|${dstPort}`;
    const newSrc = hldAssociationPortId(srcNode, "out");
    const newDst = hldAssociationPortId(dstNode, "in");
    const label = hldEdgeLabels[oldKey];
    if (label) remappedLabels[`${newSrc}|${newDst}`] = label;
  }

  return {
    ...graph,
    id: `${graph.id ?? "pir"}-hld`,
    name: `${graph.name ?? "Firmware"} — HLD`,
    description: "High-level design — services, operations, and labeled interactions",
    nodes,
    connections,
    layout: {
      ...graph.layout,
      orientation: "horizontal",
      node_width: hldLayoutNodeWidth(),
      node_gap: 56,
      layer_gap: 140,
    },
    runtime_metadata: {
      ...graph.runtime_metadata,
      overlays: {
        ...graph.runtime_metadata?.overlays,
        hld_edge_labels: remappedLabels,
        diagram_view: "hld",
      },
    },
  };
}

function buildLddGraph(
  graph: FirmwareGraph,
  pirNodes: PirNode[],
  pirEdges: PirEdge[],
  registry: NodeTypeDef[],
): FirmwareGraph {
  const pirMap = pirById(pirNodes);
  const portToNode = new Map<string, string>();
  for (const node of graph.nodes) {
    for (const port of node.ports) {
      portToNode.set(port.id, node.id);
    }
  }

  const peripheralIds = new Set(
    graph.nodes
      .filter((node) => isPeripheralNodeType(node.node_type, registry))
      .map((node) => node.id),
  );

  const nodeTypeById = new Map(graph.nodes.map((n) => [n.id, n.node_type]));
  const edgeKeys = new Set<string>();
  const workflowConnections: [string, string][] = [];
  const filteredBaseConnections: [string, string][] = [];

  for (const [srcPort, dstPort] of graph.connections) {
    const srcNode = portToNode.get(srcPort);
    const dstNode = portToNode.get(dstPort);
    if (!srcNode || !dstNode || srcNode === dstNode) continue;
    if (!peripheralIds.has(srcNode) || !peripheralIds.has(dstNode)) continue;
    const key = `${srcNode}->${dstNode}`;
    if (edgeKeys.has(key)) continue;
    edgeKeys.add(key);
    filteredBaseConnections.push([srcPort, dstPort]);
    workflowConnections.push([
      lddAssociationPortId(srcNode, "out"),
      lddAssociationPortId(dstNode, "in"),
    ]);
  }

  const nodes = graph.nodes
    .filter((node) => peripheralIds.has(node.id))
    .map((node) => {
    const pir = pirMap.get(node.id);
    return cloneNode(node, {
      label: node.label ?? node.node_type,
      description: pir?.ai_summary ?? node.description,
      ports: [
        {
          id: lddAssociationPortId(node.id, "in"),
          name: "assoc_in",
          direction: "input",
          datatype: "association",
        },
        {
          id: lddAssociationPortId(node.id, "out"),
          name: "assoc_out",
          direction: "output",
          datatype: "association",
        },
      ],
      properties: {
        ...node.properties,
        ldd_workflow: {
          role: "peripheral",
          node_type: node.node_type,
        },
      },
      visual: {
        ...node.visual,
        collapsed: false,
        layer: node.visual?.layer,
      },
      execution: node.execution,
      hardware: node.hardware,
    });
  });

  const lddEdgeLabels = buildLddEdgeLabelMap(
    filteredBaseConnections,
    portToNode,
    nodeTypeById,
    pirEdges,
  );

  const remappedLabels: Record<string, string> = {};
  for (const [srcPort, dstPort] of filteredBaseConnections) {
    const srcNode = portToNode.get(srcPort);
    const dstNode = portToNode.get(dstPort);
    if (!srcNode || !dstNode) continue;
    const oldKey = `${srcPort}|${dstPort}`;
    const newSrc = lddAssociationPortId(srcNode, "out");
    const newDst = lddAssociationPortId(dstNode, "in");
    const label = lddEdgeLabels[oldKey];
    if (label) remappedLabels[`${newSrc}|${newDst}`] = label;
  }
  if (workflowConnections.length === 0 && nodes.length > 1) {
    for (let idx = 0; idx < nodes.length - 1; idx += 1) {
      const src = nodes[idx].id;
      const dst = nodes[idx + 1].id;
      const srcPort = lddAssociationPortId(src, "out");
      const dstPort = lddAssociationPortId(dst, "in");
      workflowConnections.push([srcPort, dstPort]);
      remappedLabels[`${srcPort}|${dstPort}`] = "workflow";
    }
  }

  return {
    ...graph,
    id: `${graph.id ?? "pir"}-ldd`,
    name: `${graph.name ?? "Firmware"} — LDD`,
    description: "Peripheral workflow — hardware/component interactions only",
    nodes,
    connections: workflowConnections,
    layout: {
      ...graph.layout,
      orientation: "horizontal",
      node_width: lddLayoutNodeWidth(),
      node_gap: 48,
      layer_gap: 140,
    },
    runtime_metadata: {
      ...graph.runtime_metadata,
      overlays: {
        ...graph.runtime_metadata?.overlays,
        ldd_edge_labels: remappedLabels,
        diagram_view: "ldd",
      },
    },
  };
}

function buildSequenceGraph(graph: FirmwareGraph): FirmwareGraph {
  const { positions } = computeLayeredLayout(graph, "horizontal");
  const order = [...positions].sort((a, b) => a.layer - b.layer || a.x - b.x);
  const orderedIds = order.map((p) => p.node_id);

  const nodes = orderedIds
    .map((id) => graph.nodes.find((n) => n.id === id))
    .filter((n): n is FirmwareNode => Boolean(n))
    .map((node, idx) =>
      cloneNode(node, {
        label: `${idx + 1}. ${node.label ?? node.node_type}`,
        ports: [
          { id: portId(node.id, "in"), name: "in", direction: "input", datatype: "any" },
          { id: portId(node.id, "out"), name: "out", direction: "output", datatype: "any" },
        ],
        visual: {
          ...node.visual,
          x: idx * 320,
          y: 80,
          layer: idx,
        },
      }),
    );

  const connections: [string, string][] = [];
  for (let i = 0; i < nodes.length - 1; i += 1) {
    connections.push([
      portId(nodes[i].id, "out"),
      portId(nodes[i + 1].id, "in"),
    ]);
  }

  return {
    ...graph,
    id: `${graph.id ?? "pir"}-sequence`,
    name: `${graph.name ?? "Firmware"} — Sequence`,
    description: "Execution and initialization order",
    nodes,
    connections,
    layout: { ...graph.layout, orientation: "horizontal" },
  };
}

export function transformGraphForDiagramView(
  graph: FirmwareGraph,
  pirNodes: PirNode[],
  registry: NodeTypeDef[],
  view: GraphDiagramView,
  pirEdges: PirEdge[] = [],
): FirmwareGraph {
  switch (view) {
    case "hld":
      return buildHldGraph(graph, pirNodes, pirEdges);
    case "ldd":
      return buildLddGraph(graph, pirNodes, pirEdges, registry);
    case "sequence":
      return buildSequenceGraph(graph);
    default:
      return graph;
  }
}

export function layoutOrientationForView(view: GraphDiagramView): GraphOrientation {
  if (view === "ldd") return "vertical";
  return "horizontal";
}
