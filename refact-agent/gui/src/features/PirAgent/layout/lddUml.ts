import type { FirmwareNode } from "../types";
import type { PirEdge, PirNode } from "../pirTypes";

export type LddUmlAttribute = {
  name: string;
  type: string;
};

export type LddUmlMethod = {
  name: string;
  signature: string;
};

/** UML class compartment payload stored on LDD view nodes. */
export type LddUmlClass = {
  className: string;
  stereotype?: string;
  attributes: LddUmlAttribute[];
  methods: LddUmlMethod[];
};

const LDD_NODE_WIDTH = 300;
const LDD_ATTR_ROW = 18;
const LDD_METHOD_ROW = 18;
const LDD_HEADER = 36;
const LDD_SECTION_PAD = 8;

export function lddNodeHeight(uml: LddUmlClass): number {
  const attrRows = Math.max(uml.attributes.length, 1);
  const methodRows = Math.max(uml.methods.length, 1);
  return (
    LDD_HEADER +
    LDD_SECTION_PAD +
    attrRows * LDD_ATTR_ROW +
    LDD_SECTION_PAD +
    methodRows * LDD_METHOD_ROW +
    LDD_SECTION_PAD
  );
}

export function lddLayoutNodeWidth(): number {
  return LDD_NODE_WIDTH;
}

export function toPascalClassName(raw: string): string {
  const cleaned = raw
    .replace(/[^a-zA-Z0-9]+/g, " ")
    .trim();
  if (!cleaned) return "Component";
  return cleaned
    .split(/\s+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join("");
}

function pushAttr(
  attrs: LddUmlAttribute[],
  seen: Set<string>,
  name: string,
  type: string,
): void {
  const key = name.toLowerCase();
  if (seen.has(key)) return;
  seen.add(key);
  attrs.push({ name, type });
}

function propsObject(node: FirmwareNode): Record<string, unknown> {
  const p = node.properties;
  if (p == null || typeof p !== "object" || Array.isArray(p)) return {};
  return p;
}

export function buildLddUmlClass(node: FirmwareNode, pir?: PirNode): LddUmlClass {
  const label = node.label ?? node.node_type.replace(/_/g, " ");
  const className = toPascalClassName(label);
  const attrs: LddUmlAttribute[] = [];
  const methods: LddUmlMethod[] = [];
  const seenAttr = new Set<string>();
  const props = propsObject(node);

  const pin = props.pin ?? node.hardware?.gpio;
  if (pin != null) pushAttr(attrs, seenAttr, "pin", "gpio_num_t");
  if (node.hardware?.bus) pushAttr(attrs, seenAttr, "bus", "String");
  if (node.hardware?.i2c_address) pushAttr(attrs, seenAttr, "i2cAddress", "uint8_t");

  const typedKeys: [string, string, string][] = [
    ["task_name", "taskName", "String"],
    ["priority", "priority", "uint8_t"],
    ["stack_size", "stackSize", "uint32_t"],
    ["period_ms", "periodMs", "uint32_t"],
    ["ssid", "ssid", "String"],
    ["password", "password", "String"],
    ["broker_url", "brokerUrl", "String"],
    ["topic", "topic", "String"],
    ["target", "target", "String"],
    ["mode", "mode", "String"],
  ];
  for (const [propKey, attrName, attrType] of typedKeys) {
    if (props[propKey] != null && props[propKey] !== "") {
      pushAttr(attrs, seenAttr, attrName, attrType);
    }
  }

  if (node.execution?.phase) pushAttr(attrs, seenAttr, "phase", "String");
  if (node.execution?.period_ms != null) {
    pushAttr(attrs, seenAttr, "periodMs", "uint32_t");
  }

  const primaryFile =
    pir?.ownership.primary_files[0] ?? pir?.source_refs[0]?.file;
  if (primaryFile) {
    pushAttr(attrs, seenAttr, "sourceFile", "String");
  }

  const symbol = pir?.source_refs.find((r) => r.symbol)?.symbol;
  const nodeType = node.node_type;

  if (nodeType === "system_init" || node.id === "boot") {
    methods.push({ name: "app_main", signature: "app_main(): void" });
    methods.push({ name: "initialize", signature: "initialize(): void" });
  } else if (nodeType === "rtos_task") {
    const taskName =
      typeof props.task_name === "string" ? props.task_name : "task";
    methods.push({
      name: taskName,
      signature: `${taskName}(void* arg): void`,
    });
    methods.push({ name: "run", signature: "run(): void" });
  } else if (nodeType === "gpio_output") {
    methods.push({ name: "setLevel", signature: "setLevel(level: int): void" });
    methods.push({ name: "configure", signature: "configure(): esp_err_t" });
  } else if (nodeType === "gpio_input" || nodeType === "sensor_input") {
    methods.push({ name: "read", signature: "read(): int" });
    if (/ir|pir|motion|sensor/i.test(label)) {
      methods.push({
        name: "onMotionDetected",
        signature: "onMotionDetected(): void",
      });
    }
  } else if (nodeType === "wifi_manager") {
    methods.push({ name: "connect", signature: "connect(): bool" });
    methods.push({ name: "disconnect", signature: "disconnect(): void" });
  } else if (nodeType === "mqtt_client") {
    methods.push({
      name: "publish",
      signature: "publish(topic: String, payload: String): void",
    });
    methods.push({
      name: "subscribe",
      signature: "subscribe(topic: String): void",
    });
  } else if (symbol) {
    methods.push({
      name: symbol,
      signature: `${symbol}(): void`,
    });
  } else {
    methods.push({
      name: "init",
      signature: `init(): esp_err_t`,
    });
  }

  let stereotype: string | undefined;
  if (nodeType === "wifi_manager" || nodeType === "mqtt_client") {
    stereotype = "service";
  } else if (nodeType.startsWith("gpio_") || nodeType === "sensor_input") {
    stereotype = "peripheral";
  }

  if (attrs.length === 0) {
    pushAttr(attrs, seenAttr, "id", "String");
  }

  return { className, stereotype, attributes: attrs, methods };
}

export function lddAssociationPortId(nodeId: string, direction: "in" | "out"): string {
  return `${nodeId}::uml::${direction}`;
}

export function inferAssociationLabel(
  sourceType: string,
  targetType: string,
  edgeKind?: string,
  semanticLabel?: string,
): string {
  if (semanticLabel?.trim()) return semanticLabel.trim();

  const kind = (edgeKind ?? "").toLowerCase();
  if (kind === "execution" || kind === "fsm") {
    if (sourceType === "system_init") return "initializes";
    if (sourceType === "rtos_task" && targetType === "gpio_output") return "controls";
    if (sourceType === "rtos_task") return "invokes";
    if (sourceType === "sensor_input") return "triggers";
    return "executes";
  }
  if (kind === "data") {
    if (sourceType === "sensor_input") return "reads";
    return "dataFlow";
  }
  if (kind === "hardware") return "uses";
  if (kind === "network") return "connects";
  if (kind === "dependency") return "dependsOn";

  if (sourceType === "sensor_input" && targetType === "rtos_task") {
    return "triggers";
  }
  if (sourceType === "rtos_task" && targetType === "gpio_output") {
    return "controls";
  }
  if (sourceType === "system_init") return "bootstraps";
  if (targetType === "wifi_manager" || targetType === "mqtt_client") {
    return "uses";
  }
  return "associates";
}

export function buildLddEdgeLabelMap(
  connections: [string, string][],
  portToNode: Map<string, string>,
  nodeTypeById: Map<string, string>,
  pirEdges: PirEdge[],
): Record<string, string> {
  const labels: Record<string, string> = {};

  const pirByPair = new Map<string, PirEdge>();
  for (const edge of pirEdges) {
    const key = `${edge.source_node_id}->${edge.target_node_id}`;
    if (!pirByPair.has(key)) pirByPair.set(key, edge);
  }

  for (const [srcPort, dstPort] of connections) {
    const srcNode = portToNode.get(srcPort);
    const dstNode = portToNode.get(dstPort);
    if (!srcNode || !dstNode || srcNode === dstNode) continue;

    const connKey = `${srcPort}|${dstPort}`;
    const pirEdge = pirByPair.get(`${srcNode}->${dstNode}`);
    const srcType = nodeTypeById.get(srcNode) ?? "";
    const dstType = nodeTypeById.get(dstNode) ?? "";
    labels[connKey] = inferAssociationLabel(
      srcType,
      dstType,
      pirEdge?.kind,
      pirEdge?.semantic_label,
    );
  }

  return labels;
}
