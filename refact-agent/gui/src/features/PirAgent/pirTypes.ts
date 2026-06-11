import type { FirmwareGraph, ValidationReport } from "./types";

export type PirApprovalStatus = "pending" | "approved" | "rejected" | "stale";

/** Controlled graph lifecycle — updates only on explicit triggers. */
export type GraphLifecycleMode =
  | "idle"
  | "generating"
  | "stable"
  | "dirty"
  | "manually_refreshing";

export type PirSyncState =
  | "synced"
  | "stale"
  | "conflict"
  | "manual"
  | "analyzing";

export type NodeAuthority = "agent" | "user" | "hybrid";

export type SourceRef = {
  file: string;
  line?: number;
  symbol?: string;
  confidence: number;
  inferred_by: string;
};

export type PirNodeProperties = Record<string, unknown> & {
  // Canonical multi-pin peripheral mapping (signal/binding -> GPIO number).
  pin_bindings?: Record<string, number>;
  pin?: number;
};

export type PirNode = {
  id: string;
  node_type: string;
  label?: string;
  properties: PirNodeProperties;
  source_refs: SourceRef[];
  sync: {
    state: PirSyncState;
    last_synced_revision: string;
    last_error?: string;
  };
  ownership: {
    primary_files: string[];
    component_id?: string;
  };
  editable_fields: string[];
  layer?: string;
  ai_summary?: string;
  confidence: number;
  authority: NodeAuthority;
  semantic_tags: string[];
  dependencies: string[];
  stale_reason?: string;
};

export type PirEdge = {
  id: string;
  source_node_id: string;
  target_node_id: string;
  source_port_id?: string;
  target_port_id?: string;
  kind: string;
  confidence: number;
  semantic_label?: string;
  validated?: boolean;
};

export type PirDocument = {
  schema_version: string;
  id: string;
  revision: string;
  graph_version: number;
  provenance: {
    project_path: string;
    chat_id?: string;
    revision: string;
    generated_at_ms: number;
    analyzer_version: string;
    analyzed_files: string[];
    file_hashes?: Record<string, string>;
    board_id?: string;
  };
  approval: {
    status: PirApprovalStatus;
    approved_at_ms?: number;
    approved_revision?: string;
    comment?: string;
  };
  nodes: PirNode[];
  edges: PirEdge[];
  summary?: {
    headline: string;
    node_count: number;
    edge_count: number;
    warnings: string[];
  };
  diagrams?: {
    hld?: {
      title?: string;
      mermaid?: string;
    };
    lld?: {
      title?: string;
      mermaid?: string;
    };
    sequence?: {
      title?: string;
      mermaid?: string;
      participants?: string[];
      generated_from?: string[];
      generation_error?: string;
    };
    // Backward compatibility with older PIR snapshots.
    hld_mermaid?: string;
    lld_mermaid?: string;
    sequence_mermaid?: string;
    hld_graph?: FirmwareGraph;
    lld_graph?: FirmwareGraph;
    sequence_graph?: FirmwareGraph;
  };
  generation?: {
    mode: string;
    model?: string;
    triggered_by: string;
  };
  sync_metadata?: {
    last_diff?: TopologyDiff;
  };
  validation_state?: {
    valid: boolean;
    error_count: number;
    warning_count: number;
  };
};

export type TopologyDiff = {
  from_revision: string;
  to_revision: string;
  nodes_added: string[];
  nodes_removed: string[];
  nodes_changed: string[];
  edges_added: string[];
  edges_removed: string[];
};

export type PirAnalyzeResult = {
  status: string;
  pir: PirDocument;
  graph: FirmwareGraph;
  validation: ValidationReport;
  diff?: TopologyDiff;
};

export type PirStatus = {
  active: boolean;
  chat_id?: string;
  status: string;
  project_path?: string;
  revision?: string;
  approval_status?: PirApprovalStatus;
  watch_enabled: boolean;
  error?: string;
  summary_headline?: string;
  agent_mode?: string;
  graph_version?: number;
};

export type StructuralPatchRequest = {
  add_nodes?: PirNode[];
  remove_node_ids?: string[];
  add_edges?: PirEdge[];
  remove_edge_ids?: string[];
};
