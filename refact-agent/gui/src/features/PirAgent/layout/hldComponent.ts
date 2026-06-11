import type { FirmwareNode } from "../types";
import type { PirEdge, PirNode } from "../pirTypes";
import { inferAssociationLabel } from "./lddUml";

export type HldComponent = {
  componentName: string;
  methods: string[];
  /** Layer hint for layout ordering (left → right). */
  tier: "entry" | "control" | "io" | "connectivity" | "storage";
};

const HLD_NODE_WIDTH = 260;
const HLD_METHOD_ROW = 20;
const HLD_HEADER = 40;
const HLD_BODY_PAD = 10;

export function hldLayoutNodeWidth(): number {
  return HLD_NODE_WIDTH;
}

export function hldNodeHeight(component: HldComponent): number {
  const rows = Math.max(component.methods.length, 1);
  return HLD_HEADER + HLD_BODY_PAD + rows * HLD_METHOD_ROW + HLD_BODY_PAD;
}

export function hldAssociationPortId(nodeId: string, direction: "in" | "out"): string {
  return `${nodeId}::hld::${direction}`;
}

function propsObject(node: FirmwareNode): Record<string, unknown> {
  const p = node.properties;
  if (p == null || typeof p !== "object" || Array.isArray(p)) return {};
  return p;
}

function toServiceName(label: string, nodeType: string): string {
  const trimmed = label.trim();
  if (trimmed) return trimmed;
  return nodeType
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

function pushMethod(methods: string[], seen: Set<string>, signature: string): void {
  const key = signature.toLowerCase();
  if (seen.has(key)) return;
  seen.add(key);
  methods.push(signature);
}

function resolveTierFromType(nodeType: string, nodeId: string): HldComponent["tier"] {
  if (nodeType === "system_init" || nodeId === "boot") return "entry";
  if (/wifi|mqtt|network/i.test(nodeType)) return "connectivity";
  if (/task|rtos|timer/i.test(nodeType)) return "control";
  if (/gpio|sensor|i2c|spi|uart|adc/i.test(nodeType)) return "io";
  return "control";
}

export function buildHldComponent(node: FirmwareNode, pir?: PirNode): HldComponent {
  const label = node.label ?? node.node_type;
  const componentName = toServiceName(label, node.node_type);
  const methods: string[] = [];
  const seen = new Set<string>();
  const props = propsObject(node);
  const nodeType = node.node_type;

  if (nodeType === "system_init" || node.id === "boot") {
    pushMethod(methods, seen, "startSystem()");
    pushMethod(methods, seen, "initializeHardware()");
    pushMethod(methods, seen, "registerTasks()");
  } else if (nodeType === "rtos_task") {
    const taskName =
      typeof props.task_name === "string" ? props.task_name : "controlLoop";
    pushMethod(methods, seen, `${taskName}()`);
    pushMethod(methods, seen, "processEvents()");
    pushMethod(methods, seen, "handleTriggers()");
  } else if (nodeType === "gpio_output") {
    pushMethod(methods, seen, "setOutput()");
    pushMethod(methods, seen, "toggle()");
    pushMethod(methods, seen, "configurePin()");
  } else if (nodeType === "gpio_input" || nodeType === "sensor_input") {
    pushMethod(methods, seen, "readSample()");
    if (/ir|pir|motion/i.test(label)) {
      pushMethod(methods, seen, "onMotionDetected()");
    } else {
      pushMethod(methods, seen, "onStateChange()");
    }
    pushMethod(methods, seen, "calibrate()");
  } else if (nodeType === "wifi_manager") {
    pushMethod(methods, seen, "connectToNetwork()");
    pushMethod(methods, seen, "disconnect()");
    pushMethod(methods, seen, "getConnectionStatus()");
  } else if (nodeType === "mqtt_client") {
    pushMethod(methods, seen, "publishTelemetry()");
    pushMethod(methods, seen, "subscribeCommands()");
    pushMethod(methods, seen, "handleBrokerMessage()");
  } else if (nodeType.startsWith("i2c")) {
    pushMethod(methods, seen, "readRegister()");
    pushMethod(methods, seen, "writeRegister()");
    pushMethod(methods, seen, "probeDevice()");
  } else {
    pushMethod(methods, seen, "initialize()");
    pushMethod(methods, seen, "process()");
  }

  const symbol = pir?.source_refs.find((r) => r.symbol)?.symbol;
  if (symbol && !seen.has(`${symbol}()`.toLowerCase())) {
    pushMethod(methods, seen, `${symbol}()`);
  }

  return {
    componentName,
    methods,
    tier: resolveTierFromType(nodeType, node.id),
  };
}

/** Turn technical edge ids into HLD-style action phrases (Title Case). */
export function formatHldInteractionLabel(
  technical: string,
  sourceType: string,
  targetType: string,
  _sourceLabel: string,
  targetLabel: string,
): string {
  const key = technical.trim().toLowerCase().replace(/\s+/g, "_");

  const contextual: Record<string, string> = {
    triggers: "Forward Sensor Event",
    controls: `Control ${targetLabel}`,
    initializes: `Initialize ${targetLabel}`,
    spawns: `Start ${targetLabel}`,
    invokes: `Invoke ${targetLabel}`,
    reads: "Read Sensor Data",
    dataflow: "Transfer Data",
    uses: `Use ${targetLabel}`,
    connects: `Connect to ${targetLabel}`,
    dependson: `Depends on ${targetLabel}`,
    executes: "Execute Flow",
    bootstraps: "Bootstrap System",
    associates: `Interact with ${targetLabel}`,
  };

  if (contextual[key]) return contextual[key];

  if (sourceType === "sensor_input" && targetType === "rtos_task") {
    return "Forward Sensor Event";
  }
  if (sourceType === "rtos_task" && targetType === "gpio_output") {
    return `Drive ${targetLabel}`;
  }
  if (sourceType === "system_init" && targetType === "rtos_task") {
    return `Start ${targetLabel}`;
  }
  if (targetType === "wifi_manager") return "Connect Network";
  if (targetType === "mqtt_client") return "Publish / Subscribe";
  if (sourceType === "wifi_manager" && targetType === "mqtt_client") {
    return "Open MQTT Session";
  }

  return technical
    .replace(/_/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

export function inferHldInteractionLabel(
  sourceType: string,
  targetType: string,
  sourceLabel: string,
  targetLabel: string,
  edgeKind?: string,
  semanticLabel?: string,
): string {
  const technical = inferAssociationLabel(
    sourceType,
    targetType,
    edgeKind,
    semanticLabel,
  );
  return formatHldInteractionLabel(
    technical,
    sourceType,
    targetType,
    sourceLabel,
    targetLabel,
  );
}

export function buildHldEdgeLabelMap(
  connections: [string, string][],
  portToNode: Map<string, string>,
  nodeById: Map<string, FirmwareNode>,
  pirEdges: PirEdge[],
): Record<string, string> {
  const labels: Record<string, string> = {};
  const pirByPair = new Map<string, PirEdge>();
  for (const edge of pirEdges) {
    const key = `${edge.source_node_id}->${edge.target_node_id}`;
    if (!pirByPair.has(key)) pirByPair.set(key, edge);
  }

  for (const [srcPort, dstPort] of connections) {
    const srcNodeId = portToNode.get(srcPort);
    const dstNodeId = portToNode.get(dstPort);
    if (!srcNodeId || !dstNodeId || srcNodeId === dstNodeId) continue;

    const srcNode = nodeById.get(srcNodeId);
    const dstNode = nodeById.get(dstNodeId);
    if (!srcNode || !dstNode) continue;

    const connKey = `${srcPort}|${dstPort}`;
    const pirEdge = pirByPair.get(`${srcNodeId}->${dstNodeId}`);
    const srcLabel = srcNode.label ?? srcNode.node_type;
    const dstLabel = dstNode.label ?? dstNode.node_type;

    labels[connKey] = inferHldInteractionLabel(
      srcNode.node_type,
      dstNode.node_type,
      srcLabel,
      dstLabel,
      pirEdge?.kind,
      pirEdge?.semantic_label,
    );
  }

  return labels;
}
