import React, { useCallback, useEffect, useMemo, useRef } from "react";
import ReactFlow, {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  useReactFlow,
  type Connection,
  type Edge,
  type Node,
  useEdgesState,
  useNodesState,
  MarkerType,
} from "reactflow";
import "reactflow/dist/style.css";
import classNames from "classnames";

import type { GraphDiagramView } from "../layout/graphViewTypes";
import type {
  FirmwareGraph,
  FirmwareNode,
  FirmwareNodeData,
  NodeTypeDef,
  ValidationIssue,
} from "../types";
import {
  DEFAULT_NODE_GAP,
  DEFAULT_NODE_HEIGHT,
  DEFAULT_NODE_WIDTH,
} from "../types";
import { firmwareNodeTypes } from "./firmwareNodeTypes";
import styles from "./GraphCanvas.module.css";

export type GraphCanvasViewMode =
  | "interactive"
  | "preview"
  | "readonly"
  /** Draggable nodes, no new connections (HLD / LLD / sequence). */
  | "diagram-layout";

type GraphCanvasProps = {
  graph: FirmwareGraph;
  registry: NodeTypeDef[];
  validationIssues?: ValidationIssue[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string | null) => void;
  onGraphChange?: (graph: FirmwareGraph) => void;
  diffNodeIds?: Set<string>;
  nodeConfidence?: Map<string, number>;
  /** Collapsed chat preview: read-only, minimap-forward layout. */
  viewMode?: GraphCanvasViewMode;
  diagramView?: GraphDiagramView;
};

function readNodePosition(node: FirmwareNode, idx: number): { x: number; y: number } {
  if (
    node.visual != null &&
    typeof node.visual.x === "number" &&
    typeof node.visual.y === "number"
  ) {
    return { x: node.visual.x, y: node.visual.y };
  }
  const col = idx % 3;
  const row = Math.floor(idx / 3);
  return {
    x: col * (DEFAULT_NODE_WIDTH + DEFAULT_NODE_GAP),
    y: row * (DEFAULT_NODE_HEIGHT + DEFAULT_NODE_GAP),
  };
}

const FitViewOnLayoutChange: React.FC<{
  layoutKey: string;
  /** Preview mode: only re-fit when node topology changes, not every position tweak. */
  stablePreview?: boolean;
}> = ({ layoutKey, stablePreview = false }) => {
  const { fitView } = useReactFlow();
  const prevTopologyRef = useRef<string | null>(null);
  useEffect(() => {
    const topologyKey = stablePreview
      ? layoutKey
          .split("|")
          .map((part) => part.split(":")[0])
          .join("|")
      : layoutKey;
    if (
      stablePreview &&
      prevTopologyRef.current !== null &&
      prevTopologyRef.current === topologyKey
    ) {
      return;
    }
    prevTopologyRef.current = topologyKey;
    const timer = window.setTimeout(() => {
      try {
        void fitView({ padding: 0.18, duration: stablePreview ? 0 : 200 });
      } catch {
        /* ignore fitView before dimensions are ready */
      }
    }, 80);
    return () => window.clearTimeout(timer);
  }, [fitView, layoutKey, stablePreview]);
  return null;
};

function readDiagramEdgeLabels(
  graph: FirmwareGraph,
  view: GraphDiagramView | undefined,
): Record<string, string> {
  if (view === "ldd") {
    const raw = graph.runtime_metadata?.overlays?.ldd_edge_labels;
    if (raw && typeof raw === "object") return raw as Record<string, string>;
  }
  if (view === "hld") {
    const raw = graph.runtime_metadata?.overlays?.hld_edge_labels;
    if (raw && typeof raw === "object") return raw as Record<string, string>;
  }
  return {};
}

type PortInfo = {
  nodeId: string;
  nodeType: string;
  portName: string;
  datatype: string;
};

type EdgeSemanticKind =
  | "execution"
  | "event"
  | "network"
  | "gpio"
  | "message"
  | "data";

function normalizeDatatype(datatype?: string): string {
  return (datatype ?? "any").trim().toLowerCase();
}

function classifyEdgeKind(
  sourceDatatype: string,
  targetDatatype: string,
  sourcePortName: string,
  targetPortName: string,
): EdgeSemanticKind {
  const source = normalizeDatatype(sourceDatatype);
  const target = normalizeDatatype(targetDatatype);
  const sourcePort = sourcePortName.toLowerCase();
  const targetPort = targetPortName.toLowerCase();

  if (
    source.includes("network") ||
    target.includes("network") ||
    sourcePort.includes("network") ||
    targetPort.includes("network")
  ) {
    return "network";
  }
  if (
    source.includes("execution") ||
    target.includes("execution") ||
    sourcePort.includes("exec") ||
    targetPort.includes("exec")
  ) {
    return "execution";
  }
  if (
    source.includes("event") ||
    target.includes("event") ||
    sourcePort.includes("event") ||
    targetPort.includes("event") ||
    targetPort.includes("trigger")
  ) {
    return "event";
  }
  if (
    source.includes("gpio") ||
    target.includes("gpio") ||
    sourcePort.includes("gpio") ||
    targetPort.includes("gpio")
  ) {
    return "gpio";
  }
  if (
    source.includes("mqtt") ||
    target.includes("mqtt") ||
    source.includes("payload") ||
    target.includes("payload") ||
    source.includes("message") ||
    target.includes("message")
  ) {
    return "message";
  }
  return "data";
}

function edgeStrokeForKind(kind: EdgeSemanticKind): string {
  switch (kind) {
    case "network":
      return "var(--blue-9)";
    case "execution":
      return "var(--violet-9)";
    case "event":
      return "var(--amber-9)";
    case "gpio":
      return "var(--green-9)";
    case "message":
      return "var(--teal-9)";
    default:
      return "var(--gray-11)";
  }
}

function edgeDashForKind(kind: EdgeSemanticKind): string | undefined {
  switch (kind) {
    case "network":
      return "7 4";
    case "event":
      return "4 4";
    case "message":
      return "9 4";
    default:
      return undefined;
  }
}

function graphToFlow(
  graph: FirmwareGraph,
  registry: NodeTypeDef[],
  validationIssues: ValidationIssue[],
  selectedNodeId: string | null,
  diffNodeIds: Set<string>,
  nodeConfidence: Map<string, number>,
  diagramView?: GraphDiagramView,
): { nodes: Node<FirmwareNodeData>[]; edges: Edge[] } {
  const diagramEdgeLabels = readDiagramEdgeLabels(graph, diagramView);
  const isDiagramView =
    diagramView === "ldd" || diagramView === "hld" || diagramView === "sequence";
  const typeMap = new Map(registry.map((t) => [t.node_type, t]));
  const portInfoById = new Map<string, PortInfo>();
  for (const node of graph.nodes) {
    for (const port of node.ports) {
      portInfoById.set(port.id, {
        nodeId: node.id,
        nodeType: node.node_type,
        portName: port.name,
        datatype: normalizeDatatype(port.datatype),
      });
    }
  }

  const nodes: Node<FirmwareNodeData>[] = graph.nodes.map((node, idx) => ({
    id: node.id,
    type: "firmwareNode",
    position: readNodePosition(node, idx),
    data: {
      node,
      typeDef: typeMap.get(node.node_type),
      selected: node.id === selectedNodeId,
      validationIssues: validationIssues.filter((i) => i.node_id === node.id),
      confidence: nodeConfidence.get(node.id),
      isDiffHighlight: diffNodeIds.has(node.id),
      diagramView,
    },
    selected: node.id === selectedNodeId,
  }));

  const uniqueConnections: {
    sourceHandle: string;
    targetHandle: string;
    sourceNode: string;
    targetNode: string;
    sourcePort: PortInfo;
    targetPort: PortInfo;
    connectionIndex: number;
  }[] = [];
  const seenHandlePairs = new Set<string>();
  graph.connections.forEach(([source, target], connectionIndex) => {
    const sourcePort = portInfoById.get(source);
    const targetPort = portInfoById.get(target);
    if (!sourcePort || !targetPort) {
      return;
    }
    // Render duplicate handle pairs once to avoid visual line stacking.
    const handlePairKey = `${source}|${target}`;
    if (seenHandlePairs.has(handlePairKey)) {
      return;
    }
    seenHandlePairs.add(handlePairKey);
    uniqueConnections.push({
      sourceHandle: source,
      targetHandle: target,
      sourceNode: sourcePort.nodeId,
      targetNode: targetPort.nodeId,
      sourcePort,
      targetPort,
      connectionIndex,
    });
  });

  const pairToEdgeIndices = new Map<string, number[]>();
  uniqueConnections.forEach((connection, edgeIdx) => {
    const pairKey = `${connection.sourceNode}->${connection.targetNode}`;
    const current = pairToEdgeIndices.get(pairKey) ?? [];
    current.push(edgeIdx);
    pairToEdgeIndices.set(pairKey, current);
  });

  const edges: Edge[] = uniqueConnections.flatMap((connection, edgeIdx) => {
    if (!connection.sourceNode || !connection.targetNode) {
      return [];
    }
    const edgeIssues = validationIssues.filter(
      (issue) =>
        issue.connection &&
        issue.connection[0] === connection.sourceHandle &&
        issue.connection[1] === connection.targetHandle,
    );
    const hasError = edgeIssues.some((e) => e.severity === "error");
    const assocLabel =
      diagramEdgeLabels[`${connection.sourceHandle}|${connection.targetHandle}`];
    const label = hasError ? edgeIssues[0]?.message : assocLabel;

    const semanticKind = classifyEdgeKind(
      connection.sourcePort.datatype,
      connection.targetPort.datatype,
      connection.sourcePort.portName,
      connection.targetPort.portName,
    );
    const pairKey = `${connection.sourceNode}->${connection.targetNode}`;
    const siblingEdgeIndices = pairToEdgeIndices.get(pairKey) ?? [edgeIdx];
    const siblingPosition = siblingEdgeIndices.indexOf(edgeIdx);
    const centeredOffset =
      siblingPosition - (siblingEdgeIndices.length - 1) / 2;
    const laneMagnitude = Math.abs(centeredOffset);
    const pathOffset = 18 + laneMagnitude * 12;

    const isHldView = diagramView === "hld";
    const stroke = hasError
      ? "var(--red-9)"
      : edgeStrokeForKind(semanticKind);
    const strokeDasharray = hasError ? undefined : edgeDashForKind(semanticKind);
    const shouldAnimate = !isDiagramView && !hasError
      ? semanticKind === "network" || semanticKind === "event"
      : false;
    return [{
      id: `e-${connection.connectionIndex}-${connection.sourceHandle}-${connection.targetHandle}`,
      source: connection.sourceNode,
      target: connection.targetNode,
      sourceHandle: connection.sourceHandle,
      targetHandle: connection.targetHandle,
      type: "smoothstep",
      animated: shouldAnimate,
      markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
      pathOptions: {
        borderRadius: isDiagramView ? 18 : 12,
        offset: pathOffset,
      },
      style: {
        stroke,
        strokeWidth: isDiagramView ? 1.5 : hasError ? 2.2 : 1.6 + laneMagnitude * 0.15,
        strokeDasharray,
        opacity: hasError ? 1 : 0.95,
      },
      className: styles.topologyEdge,
      label,
      labelStyle: {
        fill: hasError ? "var(--red-11)" : isHldView ? "var(--green-11)" : "var(--gray-12)",
        fontSize: 11,
        fontWeight: 600,
      },
      labelBgStyle: isDiagramView
        ? {
            fill: isHldView ? "var(--green-3)" : "var(--gray-3)",
            fillOpacity: 1,
          }
        : undefined,
      labelBgPadding: isDiagramView ? ([4, 6] as [number, number]) : undefined,
      labelBgBorderRadius: isDiagramView ? 4 : undefined,
      interactionWidth: 22,
    }];
  });

  return { nodes, edges };
}

export const GraphCanvas: React.FC<GraphCanvasProps> = ({
  graph,
  registry,
  validationIssues = [],
  selectedNodeId,
  onSelectNode,
  onGraphChange,
  diffNodeIds = new Set(),
  nodeConfidence = new Map<string, number>(),
  viewMode = "interactive",
  diagramView,
}) => {
  const isPreview = viewMode === "preview";
  const isReadonly = viewMode === "readonly";
  const isDiagramLayout = viewMode === "diagram-layout";
  const allowDrag = !isPreview;
  const allowConnect = !isPreview && !isReadonly && !isDiagramLayout;
  const mapped = useMemo(
    () =>
      graphToFlow(
        graph,
        registry,
        validationIssues,
        selectedNodeId,
        diffNodeIds,
        nodeConfidence,
        diagramView,
      ),
    [
      graph,
      registry,
      validationIssues,
      selectedNodeId,
      diffNodeIds,
      nodeConfidence,
      diagramView,
    ],
  );

  const [nodes, setNodes, onNodesChange] = useNodesState(mapped.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(mapped.edges);
  const mappedRef = useRef(mapped);
  mappedRef.current = mapped;

  const preserveDraggedPositions = allowDrag || isDiagramLayout;

  const topologyKey = useMemo(() => {
    const orientation = graph.layout?.orientation ?? "horizontal";
    const nodeIds = [...graph.nodes.map((n) => n.id)].sort().join(",");
    return `${diagramView ?? "topology"}|${orientation}|${nodeIds}|${graph.connections.length}`;
  }, [graph.nodes, graph.connections.length, graph.layout?.orientation, diagramView]);

  const graphContentKey = useMemo(
    () =>
      graph.nodes
        .map(
          (n) =>
            `${n.id}:${n.node_type}:${JSON.stringify(n.properties)}:${n.label ?? ""}`,
        )
        .join("|"),
    [graph.nodes],
  );

  const fitViewLayoutKey = useMemo(() => {
    if (isDiagramLayout || isPreview) {
      return `${diagramView ?? "diagram"}|${graph.nodes
        .map((n) => n.id)
        .sort()
        .join("|")}`;
    }
    return topologyKey;
  }, [isDiagramLayout, isPreview, diagramView, graph.nodes, topologyKey]);

  useEffect(() => {
    const nextNodes = mappedRef.current.nodes;
    const nextEdges = mappedRef.current.edges;
    if (preserveDraggedPositions) {
      setNodes((current) => {
        const byId = new Map(current.map((n) => [n.id, n]));
        return nextNodes.map((mn) => {
          const cur = byId.get(mn.id);
          if (cur) return { ...mn, position: cur.position };
          return mn;
        });
      });
    } else {
      setNodes(nextNodes);
    }
    setEdges(nextEdges);
  }, [topologyKey, preserveDraggedPositions, setNodes, setEdges]);

  useEffect(() => {
    if (!preserveDraggedPositions) return;
    setNodes((current) => {
      const byId = new Map(current.map((n) => [n.id, n]));
      return mappedRef.current.nodes.map((mn) => {
        const cur = byId.get(mn.id);
        if (cur) return { ...mn, position: cur.position };
        return mn;
      });
    });
  }, [graphContentKey, preserveDraggedPositions, setNodes]);

  useEffect(() => {
    setNodes((current) =>
      current.map((n) => ({
        ...n,
        selected: n.id === selectedNodeId,
        data: {
          ...n.data,
          selected: n.id === selectedNodeId,
        },
      })),
    );
  }, [selectedNodeId, setNodes]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node<FirmwareNodeData>) => {
      onSelectNode(node.id);
    },
    [onSelectNode],
  );

  const onPaneClick = useCallback(() => {
    onSelectNode(null);
  }, [onSelectNode]);

  const onNodeDragStop = useCallback(
    (_: React.MouseEvent, node: Node<FirmwareNodeData>) => {
      if (!onGraphChange) return;
      const updated: FirmwareGraph = {
        ...graph,
        nodes: graph.nodes.map((n) =>
          n.id === node.id
            ? {
                ...n,
                visual: {
                  ...n.visual,
                  x: node.position.x,
                  y: node.position.y,
                },
              }
            : n,
        ),
      };
      onGraphChange(updated);
    },
    [graph, onGraphChange],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      if (!connection.sourceHandle || !connection.targetHandle || !onGraphChange)
        return;
      if (
        graph.connections.some(
          ([source, target]) =>
            source === connection.sourceHandle &&
            target === connection.targetHandle,
        )
      ) {
        return;
      }
      const updated: FirmwareGraph = {
        ...graph,
        connections: [
          ...graph.connections,
          [connection.sourceHandle, connection.targetHandle],
        ],
      };
      onGraphChange(updated);
    },
    [graph, onGraphChange],
  );

  return (
    <div
      className={classNames(
        styles.canvas,
        isPreview && styles.canvasPreview,
        diagramView === "hld" && styles.canvasHld,
        diagramView === "ldd" && styles.canvasLdd,
        diagramView === "sequence" && styles.canvasSequence,
      )}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={allowDrag ? onNodesChange : undefined}
        onEdgesChange={allowConnect ? onEdgesChange : undefined}
        onNodeClick={isPreview ? undefined : onNodeClick}
        onPaneClick={isPreview ? undefined : onPaneClick}
        onNodeDragStop={allowDrag ? onNodeDragStop : undefined}
        onConnect={allowConnect ? onConnect : undefined}
        nodeTypes={firmwareNodeTypes}
        nodesDraggable={allowDrag}
        nodesConnectable={allowConnect}
        elementsSelectable={!isPreview}
        panOnDrag={!isPreview}
        zoomOnScroll={!isPreview}
        zoomOnPinch={!isPreview}
        zoomOnDoubleClick={false}
        preventScrolling={isPreview}
        fitView
        fitViewOptions={{ padding: isPreview ? 0.08 : 0.2 }}
        minZoom={isPreview ? 0.05 : 0.2}
        maxZoom={isPreview ? 0.5 : isReadonly || isDiagramLayout ? 1.5 : 2}
        proOptions={{ hideAttribution: true }}
      >
        <FitViewOnLayoutChange
          layoutKey={fitViewLayoutKey}
          stablePreview={isPreview || isDiagramLayout}
        />
        <MiniMap
          className={isPreview ? styles.minimapPreview : styles.minimap}
          nodeColor={(n) =>
            (n.data as FirmwareNodeData).typeDef?.color ?? "#607080"
          }
          maskColor="rgba(0,0,0,0.55)"
          pannable={!isPreview}
          zoomable={!isPreview}
        />
        {!isPreview ? (
          <>
            <Controls className={styles.controls} />
            <Background
              variant={BackgroundVariant.Dots}
              gap={16}
              size={1}
              color="var(--gray-6)"
            />
          </>
        ) : null}
      </ReactFlow>
    </div>
  );
};
