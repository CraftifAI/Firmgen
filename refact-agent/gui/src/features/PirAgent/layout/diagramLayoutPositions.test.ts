import { describe, expect, it } from "vitest";

import type { FirmwareGraph } from "../types";
import {
  applyDiagramPositions,
  extractDiagramPositions,
} from "./diagramLayoutPositions";

describe("diagramLayoutPositions", () => {
  const graph: FirmwareGraph = {
    schema_version: 1,
    nodes: [
      { id: "a", node_type: "gpio_output", ports: [], visual: { x: 10, y: 20 } },
      { id: "b", node_type: "rtos_task", ports: [] },
    ],
    connections: [],
  };

  it("extracts and reapplies saved positions", () => {
    const saved = extractDiagramPositions(graph);
    expect(saved.a).toEqual({ x: 10, y: 20 });
    const next = applyDiagramPositions(graph, { b: { x: 100, y: 200 } });
    expect(next.nodes.find((n) => n.id === "b")?.visual).toEqual({ x: 100, y: 200 });
    expect(next.nodes.find((n) => n.id === "a")?.visual).toEqual({ x: 10, y: 20 });
  });
});
