import React from "react";
import { Select, Text } from "@radix-ui/themes";

import {
  GRAPH_DIAGRAM_VIEW_OPTIONS,
  type GraphDiagramView,
} from "../layout/graphViewTypes";
import styles from "./GraphDiagramViewSelect.module.css";

export type GraphDiagramViewSelectProps = {
  value: GraphDiagramView;
  onChange: (view: GraphDiagramView) => void;
  disabled?: boolean;
};

export const GraphDiagramViewSelect: React.FC<GraphDiagramViewSelectProps> = ({
  value,
  onChange,
  disabled = false,
}) => {
  const active = GRAPH_DIAGRAM_VIEW_OPTIONS.find((o) => o.value === value);

  return (
    <div className={styles.wrap}>
      <Select.Root
        value={value}
        onValueChange={(v) => onChange(v as GraphDiagramView)}
        disabled={disabled}
      >
        <Select.Trigger className={styles.trigger} aria-label="Diagram view type">
          {active?.label ?? "Diagram view"}
        </Select.Trigger>
        <Select.Content position="popper" sideOffset={4}>
          {GRAPH_DIAGRAM_VIEW_OPTIONS.map((opt) => (
            <Select.Item key={opt.value} value={opt.value}>
              <Text as="span" weight="medium">
                {opt.label}
              </Text>
            </Select.Item>
          ))}
        </Select.Content>
      </Select.Root>
      {active ? (
        <Text size="1" color="gray" className={styles.hint}>
          {active.description}
        </Text>
      ) : null}
    </div>
  );
};
