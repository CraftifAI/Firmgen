import React, { memo } from "react";
import { Handle, Position, type NodeProps } from "reactflow";
import { Badge, Flex, Text } from "@radix-ui/themes";
import classNames from "classnames";

import type {
  FirmwareNodeData,
  HldComponentView,
  LddUmlClassView,
  PortDirection,
} from "../types";
import styles from "./FirmwareNode.module.css";

function readHldComponent(node: FirmwareNodeData["node"]): HldComponentView | null {
  const raw = node.properties?.hld_component;
  if (!raw || typeof raw !== "object") return null;
  const h = raw as HldComponentView;
  if (!h.componentName || !Array.isArray(h.methods)) return null;
  return h;
}

function readLddUml(node: FirmwareNodeData["node"]): LddUmlClassView | null {
  const raw = node.properties?.ldd_uml;
  if (!raw || typeof raw !== "object") return null;
  const u = raw as LddUmlClassView;
  if (!u.className || !Array.isArray(u.attributes) || !Array.isArray(u.methods)) {
    return null;
  }
  return u;
}

function portPosition(
  direction: PortDirection,
  orientation: "horizontal" | "vertical",
): Position {
  if (orientation === "vertical") {
    return direction === "input" ? Position.Top : Position.Bottom;
  }
  return direction === "input" ? Position.Left : Position.Right;
}

function portOffset(index: number, total: number): string {
  if (total <= 1) return "50%";
  const pct = ((index + 1) / (total + 1)) * 100;
  return `${pct}%`;
}

export const FirmwareNodeView: React.FC<NodeProps<FirmwareNodeData>> = memo(
  ({ data, selected }) => {
    const {
      node,
      typeDef,
      validationIssues = [],
      confidence,
      isDiffHighlight,
      diagramView,
    } = data;
    const color = typeDef?.color ?? "#607080";
    const label = node.label ?? typeDef?.label ?? node.node_type;
    const hasError = validationIssues.some((i) => i.severity === "error");
    const hasWarning = validationIssues.some((i) => i.severity === "warning");
    const isLdd = diagramView === "ldd";
    const isHld = diagramView === "hld";
    const orientation: "horizontal" | "vertical" = isLdd ? "vertical" : "horizontal";
    const lddUml = isLdd ? readLddUml(node) : null;
    const hldComponent = isHld ? readHldComponent(node) : null;

    const inputPorts = node.ports.filter((p) => p.direction === "input");
    const outputPorts = node.ports.filter((p) => p.direction === "output");

    const runtimeBadge = node.runtime_state ?? "unknown";
    const pin = node.properties?.pin ?? node.hardware?.gpio;

    if (hldComponent) {
      return (
        <div
          className={classNames(styles.node, styles.hldNode, {
            [styles.selected]: selected,
            [styles.error]: hasError,
            [styles.warning]: !hasError && hasWarning,
          })}
        >
          <div className={styles.hldHeader}>
            <span className={styles.hldIcon} aria-hidden>
              C
            </span>
            <Text size="2" weight="bold" className={styles.hldComponentName}>
              {hldComponent.componentName}
            </Text>
          </div>

          <div className={styles.hldBody}>
            {hldComponent.methods.map((method) => (
              <Text key={method} size="1" className={styles.hldMethod}>
                {method}
              </Text>
            ))}
          </div>

          {inputPorts.map((port, i) => (
            <Handle
              key={port.id}
              id={port.id}
              type="target"
              position={Position.Left}
              className={classNames(styles.handle, styles.handleIn)}
              style={{ top: portOffset(i, inputPorts.length) }}
              title={port.name}
            />
          ))}

          {outputPorts.map((port, i) => (
            <Handle
              key={port.id}
              id={port.id}
              type="source"
              position={Position.Right}
              className={classNames(styles.handle, styles.handleOut)}
              style={{ top: portOffset(i, outputPorts.length) }}
              title={port.name}
            />
          ))}
        </div>
      );
    }

    if (lddUml) {
      return (
        <div
          className={classNames(styles.node, styles.umlNode, {
            [styles.selected]: selected,
            [styles.error]: hasError,
            [styles.warning]: !hasError && hasWarning,
          })}
        >
          <div className={styles.umlHeader}>
            {lddUml.stereotype ? (
              <Text size="1" className={styles.umlStereotype}>
                «{lddUml.stereotype}»
              </Text>
            ) : null}
            <Text size="2" weight="bold" className={styles.umlClassName}>
              {lddUml.className}
            </Text>
          </div>

          <div className={styles.umlSection}>
            {lddUml.attributes.map((attr) => (
              <div key={attr.name} className={styles.umlMember}>
                <span className={styles.umlAttrMarker} aria-hidden />
                <Text size="1" className={styles.umlMemberText}>
                  {attr.name}: {attr.type}
                </Text>
              </div>
            ))}
          </div>

          <div className={styles.umlSection}>
            {lddUml.methods.map((method) => (
              <div key={method.signature} className={styles.umlMember}>
                <span className={styles.umlMethodMarker} aria-hidden />
                <Text size="1" className={styles.umlMemberText}>
                  {method.signature}
                </Text>
              </div>
            ))}
          </div>

          {inputPorts.map((port, i) => (
            <Handle
              key={port.id}
              id={port.id}
              type="target"
              position={Position.Top}
              className={classNames(styles.handle, styles.handleIn)}
              style={{
                left: portOffset(i, inputPorts.length),
              }}
              title={port.name}
            />
          ))}

          {outputPorts.map((port, i) => (
            <Handle
              key={port.id}
              id={port.id}
              type="source"
              position={Position.Bottom}
              className={classNames(styles.handle, styles.handleOut)}
              style={{
                left: portOffset(i, outputPorts.length),
              }}
              title={port.name}
            />
          ))}
        </div>
      );
    }

    return (
      <div
        className={classNames(styles.node, {
          [styles.selected]: selected,
          [styles.error]: hasError,
          [styles.warning]: !hasError && hasWarning,
          [styles.diffHighlight]: isDiffHighlight,
        })}
      >
        <div className={styles.header} style={{ background: color }}>
          <Text size="1" weight="bold" className={styles.headerText}>
            {label}
          </Text>
          <Text size="1" className={styles.typeLabel}>
            {node.node_type.replace(/_/g, " ").toUpperCase()}
          </Text>
        </div>

        <div className={styles.body}>
          {node.description ? (
            <Text size="1" color="gray" className={styles.description}>
              {node.description}
            </Text>
          ) : null}
          <Flex gap="1" wrap="wrap" mt="1">
            {node.execution?.phase ? (
              <Badge size="1" variant="soft" color="blue">
                {node.execution.phase}
              </Badge>
            ) : null}
            {node.execution?.trigger ? (
              <Badge size="1" variant="soft" color="orange">
                {node.execution.trigger}
              </Badge>
            ) : null}
            {pin != null ? (
              <Badge size="1" variant="soft" color="green">
                GPIO {String(pin)}
              </Badge>
            ) : null}
            {confidence != null && confidence < 1 ? (
              <Badge size="1" variant="soft" color="purple">
                conf {(confidence * 100).toFixed(0)}%
              </Badge>
            ) : null}
            <Badge
              size="1"
              variant="soft"
              color={
                runtimeBadge === "running"
                  ? "green"
                  : runtimeBadge === "error"
                    ? "red"
                    : "gray"
              }
            >
              {runtimeBadge}
            </Badge>
          </Flex>
        </div>

        {inputPorts.map((port, i) => (
          <Handle
            key={port.id}
            id={port.id}
            type="target"
            position={portPosition("input", orientation)}
            className={classNames(styles.handle, styles.handleIn)}
            style={{
              [orientation === "horizontal" ? "top" : "left"]: portOffset(
                i,
                inputPorts.length,
              ),
            }}
            title={`${port.name} (${port.datatype ?? "any"})`}
          />
        ))}

        {outputPorts.map((port, i) => (
          <Handle
            key={port.id}
            id={port.id}
            type="source"
            position={portPosition("output", orientation)}
            className={classNames(styles.handle, styles.handleOut)}
            style={{
              [orientation === "horizontal" ? "top" : "left"]: portOffset(
                i,
                outputPorts.length,
              ),
            }}
            title={`${port.name} (${port.datatype ?? "any"})`}
          />
        ))}
      </div>
    );
  },
);

FirmwareNodeView.displayName = "FirmwareNodeView";
