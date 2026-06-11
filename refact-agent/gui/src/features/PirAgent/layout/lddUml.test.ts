import { describe, expect, it } from "vitest";

import type { FirmwareNode } from "../types";
import { buildLddUmlClass, inferAssociationLabel, toPascalClassName } from "./lddUml";

describe("lddUml", () => {
  it("converts labels to PascalCase class names", () => {
    expect(toPascalClassName("status led")).toBe("StatusLed");
    expect(toPascalClassName("WiFi Manager")).toBe("WifiManager");
  });

  it("builds attributes and methods for a GPIO output", () => {
    const node: FirmwareNode = {
      id: "led_out",
      node_type: "gpio_output",
      label: "Status LED",
      ports: [],
      properties: { pin: 5 },
      hardware: { gpio: 5 },
    };
    const uml = buildLddUmlClass(node);
    expect(uml.className).toBe("StatusLed");
    expect(uml.attributes.some((a) => a.name === "pin")).toBe(true);
    expect(uml.methods.some((m) => m.name === "setLevel")).toBe(true);
  });

  it("infers association labels from edge kind", () => {
    expect(
      inferAssociationLabel("sensor_input", "rtos_task", "data", undefined),
    ).toBe("reads");
    expect(
      inferAssociationLabel("rtos_task", "gpio_output", "execution", undefined),
    ).toBe("controls");
    expect(
      inferAssociationLabel("sensor_input", "rtos_task", undefined, "motion_trigger"),
    ).toBe("motion_trigger");
  });
});
