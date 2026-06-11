/** Diagram views available in the full-screen PIR editor. */
export type GraphDiagramView = "topology" | "hld" | "ldd" | "sequence";

export const GRAPH_DIAGRAM_VIEW_OPTIONS: {
  value: GraphDiagramView;
  label: string;
  description: string;
}[] = [
  {
    value: "topology",
    label: "Wiring topology",
    description: "Interactive node graph — edit variables in the inspector",
  },
  {
    value: "hld",
    label: "HLD",
    description:
      "High-level architecture — ESP32 controller, major modules/peripherals, cloud/mobile integrations, and data flow",
  },
  {
    value: "ldd",
    label: "LLD",
    description:
      "Peripheral workflow — hardware/component interactions only",
  },
  {
    value: "sequence",
    label: "Sequence",
    description: "LLM-generated Mermaid runtime interaction timeline",
  },
];

/** Inspector / property editing — topology only. */
export function isInspectorReadOnlyDiagramView(view: GraphDiagramView): boolean {
  return view !== "topology";
}

/** @deprecated Use isInspectorReadOnlyDiagramView */
export function isReadOnlyDiagramView(view: GraphDiagramView): boolean {
  return isInspectorReadOnlyDiagramView(view);
}

/** HLD and LLD support drag-to-reposition (layout stored per view). */
export function isDiagramLayoutView(view: GraphDiagramView): boolean {
  return view === "hld" || view === "ldd";
}
