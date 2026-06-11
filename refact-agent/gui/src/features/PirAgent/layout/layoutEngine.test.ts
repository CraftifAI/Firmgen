import { describe, expect, it } from "vitest";

import type { FirmwareGraph } from "../types";
import { applyLayoutToGraph, computeLayeredLayout } from "./layoutEngine";

const miniGraph: FirmwareGraph = {
  schema_version: 1,
  nodes: [
    {
      id: "a",
      node_type: "system_init",
      ports: [{ id: "a_out_0", name: "boot_out", direction: "output" }],
    },
    {
      id: "b",
      node_type: "gpio_output",
      ports: [{ id: "b_in_0", name: "exec_in", direction: "input" }],
    },
  ],
  connections: [["a_out_0", "b_in_0"]],
};

describe("computeLayeredLayout", () => {
  it("assigns boot node to layer 0 and downstream to layer 1", () => {
    const { positions, hasCycles } = computeLayeredLayout(miniGraph, "horizontal");
    expect(hasCycles).toBe(false);
    const a = positions.find((p) => p.node_id === "a");
    const b = positions.find((p) => p.node_id === "b");
    expect(a?.layer).toBe(0);
    expect(b?.layer).toBe(1);
    expect((b?.x ?? 0) > (a?.x ?? 0)).toBe(true);
  });

  it("writes visual metadata via applyLayoutToGraph", () => {
    const { positions } = computeLayeredLayout(miniGraph, "horizontal");
    const laidOut = applyLayoutToGraph(miniGraph, positions);
    expect(laidOut.nodes[0].visual?.x).toBeDefined();
    expect(laidOut.nodes[1].visual?.layer).toBe(1);
  });
});
