export type PortDirection = "input" | "output";

export type SignalMetadata = {
  dtype?: string;
  shape?: number[];
  unit?: string;
  rate_hz?: number;
  encoding?: string;
};

export type HardwareMetadata = {
  gpio?: number;
  bus?: string;
  peripheral?: string;
  pin_label?: string;
  i2c_address?: string;
  spi_host?: string;
  uart_port?: number;
};

export type Port = {
  id: string;
  name: string;
  direction: PortDirection;
  datatype?: string;
  signal?: SignalMetadata;
  hardware?: HardwareMetadata;
  required?: boolean;
  multiplicity?: "one" | "many";
};

export type ExecutionMetadata = {
  phase?: string;
  priority?: number;
  stack_size?: number;
  core_affinity?: number;
  period_ms?: number;
  trigger?: string;
};

export type VisualMetadata = {
  x?: number;
  y?: number;
  layer?: number;
  collapsed?: boolean;
};

export type FirmwareNode = {
  id: string;
  node_type: string;
  label?: string;
  description?: string;
  ports: Port[];
  properties?: Record<string, unknown>;
  hardware?: HardwareMetadata;
  execution?: ExecutionMetadata;
  visual?: VisualMetadata;
  validation_state?: "valid" | "warning" | "error" | "unknown";
  runtime_state?: "idle" | "running" | "blocked" | "error" | "unknown";
};

export type LayoutConfig = {
  orientation?: "horizontal" | "vertical";
  node_width?: number;
  layer_gap?: number;
  node_gap?: number;
};

export type RuntimeMetadata = {
  telemetry_enabled?: boolean;
  last_updated_ms?: number;
  /** Diagram view overlays (edge labels, view id, etc.). */
  overlays?: Record<string, unknown>;
};

export type FirmwareGraph = {
  schema_version: number;
  id?: string;
  name?: string;
  description?: string;
  board_id?: string;
  nodes: FirmwareNode[];
  connections: [string, string][];
  layout?: LayoutConfig;
  runtime_metadata?: RuntimeMetadata;
};

export type ValidationIssue = {
  code: string;
  message: string;
  severity: "error" | "warning" | "info";
  node_id?: string;
  port_id?: string;
  connection?: [string, string];
};

export type ValidationReport = {
  valid: boolean;
  issues: ValidationIssue[];
};

export type PropertyFieldDef = {
  key: string;
  label: string;
  type:
    | "string"
    | "number"
    | "integer"
    | "boolean"
    | "enum"
    | "url"
    | "path"
    | "json"
    | "gpio";
  default?: unknown;
  options?: string[];
  min?: number;
  max?: number;
  read_only?: boolean;
  description?: string;
};

export type NodeTypeDef = {
  node_type: string;
  category: string;
  label: string;
  color: string;
  icon: string;
  description: string;
  ports: {
    name: string;
    direction: PortDirection;
    datatype: string;
    required?: boolean;
  }[];
  properties: PropertyFieldDef[];
  execution_semantics: {
    phase: string;
    trigger?: string;
    description: string;
  };
};

export type LayoutPosition = {
  node_id: string;
  x: number;
  y: number;
  layer: number;
};

export type LayoutResult = {
  positions: LayoutPosition[];
  has_cycles: boolean;
  orientation: string;
};

export type GraphOrientation = "horizontal" | "vertical";

export const SCHEMA_VERSION = 1;

export const DEFAULT_NODE_WIDTH = 280;
export const DEFAULT_NODE_HEIGHT = 200;
export const DEFAULT_LAYER_GAP = 120;
export const DEFAULT_NODE_GAP = 100;

export type LddUmlClassView = {
  className: string;
  stereotype?: string;
  attributes: { name: string; type: string }[];
  methods: { name: string; signature: string }[];
};

export type HldComponentView = {
  componentName: string;
  methods: string[];
  tier?: string;
};

export type FirmwareNodeData = {
  node: FirmwareNode;
  typeDef?: NodeTypeDef;
  selected?: boolean;
  validationIssues?: ValidationIssue[];
  confidence?: number;
  isDiffHighlight?: boolean;
  /** When set to `ldd`, renders a UML class compartment node. */
  diagramView?: "topology" | "hld" | "ldd" | "sequence";
};
