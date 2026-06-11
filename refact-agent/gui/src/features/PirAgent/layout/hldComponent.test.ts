import { describe, expect, it } from "vitest";

import type { FirmwareNode } from "../types";
import {
  buildHldComponent,
  formatHldInteractionLabel,
  inferHldInteractionLabel,
} from "./hldComponent";

describe("hldComponent", () => {
  it("builds a service component with high-level methods", () => {
    const node: FirmwareNode = {
      id: "wifi",
      node_type: "wifi_manager",
      label: "WiFi Manager",
      ports: [],
      properties: { ssid: "home" },
    };
    const hld = buildHldComponent(node);
    expect(hld.componentName).toBe("WiFi Manager");
    expect(hld.methods).toContain("connectToNetwork()");
    expect(hld.tier).toBe("connectivity");
  });

  it("formats interaction labels for HLD arrows", () => {
    expect(
      formatHldInteractionLabel("triggers", "sensor_input", "rtos_task", "PIR", "Main Task"),
    ).toBe("Forward Sensor Event");
    expect(
      inferHldInteractionLabel(
        "rtos_task",
        "gpio_output",
        "Main Task",
        "Status LED",
        "execution",
        "controls",
      ),
    ).toBe("Control Status LED");
  });
});
