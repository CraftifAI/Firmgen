import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  Box,
  Button,
  Flex,
  Select,
  Switch,
  Text,
  TextArea,
  TextField,
} from "@radix-ui/themes";

import type {
  FirmwareNode,
  NodeTypeDef,
  PropertyFieldDef,
  ValidationIssue,
} from "../types";
import type { PirNode } from "../pirTypes";
import styles from "./NodeInspector.module.css";

type NodeInspectorProps = {
  node: FirmwareNode | null;
  typeDef: NodeTypeDef | null;
  pirNode?: PirNode | null;
  issues: ValidationIssue[];
  onApply: (nodeId: string, updated: FirmwareNode) => void;
  onClose: () => void;
  /** When true, debounce property edits and sync to project via PIR_maker. */
  syncToProject?: boolean;
};

function getNestedValue(obj: Record<string, unknown>, key: string): unknown {
  return obj[key];
}

function setNestedValue(
  obj: Record<string, unknown>,
  key: string,
  value: unknown,
): Record<string, unknown> {
  return { ...obj, [key]: value };
}

function cloneFirmwareNode(node: FirmwareNode): FirmwareNode {
  try {
    return structuredClone(node);
  } catch {
    return JSON.parse(JSON.stringify(node)) as FirmwareNode;
  }
}

const FieldEditor: React.FC<{
  field: PropertyFieldDef;
  value: unknown;
  onChange: (value: unknown) => void;
}> = ({ field, value, onChange }) => {
  if (field.read_only) {
    return (
      <Text size="1" color="gray">
        {value != null ? String(value) : "—"}
      </Text>
    );
  }

  switch (field.type) {
    case "boolean":
      return (
        <Switch
          checked={Boolean(value)}
          onCheckedChange={(checked) => onChange(checked)}
        />
      );
    case "enum": {
      const options = field.options ?? [];
      const strVal = value != null ? String(value) : "";
      const selectValue = options.includes(strVal) ? strVal : undefined;
      return (
        <Select.Root
          value={selectValue}
          onValueChange={(v) => onChange(v)}
        >
          <Select.Trigger className={styles.fieldControl} placeholder="Select…" />
          <Select.Content>
            {options.map((opt) => (
              <Select.Item key={opt} value={opt}>
                {opt}
              </Select.Item>
            ))}
          </Select.Content>
        </Select.Root>
      );
    }
    case "integer":
    case "gpio":
    case "number":
      return (
        <TextField.Root
          type="number"
          className={styles.fieldControl}
          value={value != null ? String(value) : ""}
          min={field.min}
          max={field.max}
          onChange={(e) => {
            const raw = e.target.value;
            if (raw === "") {
              onChange(undefined);
              return;
            }
            onChange(
              field.type === "integer" || field.type === "gpio"
                ? parseInt(raw, 10)
                : parseFloat(raw),
            );
          }}
        />
      );
    case "json":
      return (
        <TextArea
          className={styles.fieldControl}
          rows={4}
          value={
            typeof value === "string"
              ? value
              : JSON.stringify(value ?? {}, null, 2)
          }
          onChange={(e) => {
            try {
              onChange(JSON.parse(e.target.value));
            } catch {
              onChange(e.target.value);
            }
          }}
        />
      );
    default:
      return (
        <TextField.Root
          className={styles.fieldControl}
          value={value != null ? String(value) : ""}
          onChange={(e) => onChange(e.target.value)}
        />
      );
  }
};

export const NodeInspector: React.FC<NodeInspectorProps> = ({
  node,
  typeDef,
  pirNode,
  issues,
  onApply,
  onClose,
  syncToProject = false,
}) => {
  const [draft, setDraft] = useState<FirmwareNode | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    setDraft(node ? cloneFirmwareNode(node) : null);
    setDirty(false);
  }, [node]);

  const nodeIssues = useMemo(
    () => (node ? issues.filter((i) => i.node_id === node.id) : []),
    [issues, node],
  );

  const fields = useMemo(
    () => typeDef?.properties ?? [],
    [typeDef?.properties],
  );

  const editableKeys = useMemo(() => {
    if (pirNode?.editable_fields && pirNode.editable_fields.length > 0) {
      return new Set(pirNode.editable_fields);
    }
    return new Set(fields.filter((f) => !f.read_only).map((f) => f.key));
  }, [pirNode?.editable_fields, fields]);

  const lockedByValidation = useMemo(() => {
    const locked = new Set<string>();
    // Only lock pins the board profile marks restricted (backend strips pin from editable_fields).
    if (pirNode?.stale_reason?.toLowerCase().includes("restricted")) {
      locked.add("pin");
    }
    return locked;
  }, [pirNode?.stale_reason]);


  const updateProperty = useCallback((key: string, value: unknown) => {
    setDraft((prev) => {
      if (!prev) return prev;
      const properties = setNestedValue(
        (prev.properties ?? {}),
        key,
        value,
      );
      return { ...prev, properties };
    });
    setDirty(true);
  }, []);

  const handleApply = useCallback(() => {
    if (!draft) return;
    onApply(draft.id, draft);
    setDirty(false);
  }, [draft, onApply]);

  const [syncPending, setSyncPending] = useState(false);

  useEffect(() => {
    if (!syncToProject || !dirty || !draft) {
      setSyncPending(false);
      return;
    }
    setSyncPending(true);
    const timer = window.setTimeout(() => {
      onApply(draft.id, draft);
      setDirty(false);
      setSyncPending(false);
    }, 700);
    return () => {
      window.clearTimeout(timer);
    };
  }, [syncToProject, dirty, draft, onApply]);

  const handleRevert = useCallback(() => {
    setDraft(node ? cloneFirmwareNode(node) : null);
    setDirty(false);
  }, [node]);

  if (!node || !draft) {
    return (
      <Box className={styles.inspector}>
        <Text size="2" color="gray">
          Select a node to inspect its properties, ports, and runtime metadata.
        </Text>
      </Box>
    );
  }

  return (
    <Box className={styles.inspector}>
      <Flex justify="between" align="center" mb="3">
        <Text size="3" weight="bold">
          Inspector
        </Text>
        <Button size="1" variant="ghost" onClick={onClose}>
          Close
        </Button>
      </Flex>

      <Flex direction="column" gap="2" mb="4">
        <Text size="2" weight="medium">
          {draft.label ?? typeDef?.label ?? draft.node_type}
        </Text>
        <Text size="1" color="gray">
          {typeDef?.description ?? draft.description ?? ""}
        </Text>
        {pirNode?.ai_summary ? (
          <Text size="1" color="gray">
            AI: {pirNode.ai_summary}
          </Text>
        ) : null}
        {pirNode?.confidence != null ? (
          <Text size="1" color="gray">
            Confidence: {(pirNode.confidence * 100).toFixed(0)}% · authority:{" "}
            {pirNode.authority}
          </Text>
        ) : null}
        {nodeIssues.length > 0 ? (
          <Box className={styles.issueList}>
            {nodeIssues.map((issue) => (
              <Text
                key={`${issue.code}-${issue.message}`}
                size="1"
                color={issue.severity === "error" ? "red" : "amber"}
              >
                {issue.message}
              </Text>
            ))}
          </Box>
        ) : null}
      </Flex>

      <Text size="1" weight="bold" mb="2" className={styles.sectionTitle}>
        Properties
      </Text>
      <Flex direction="column" gap="3" mb="4">
        {fields.map((field) => {
          const locked =
            Boolean(field.read_only) ||
            !editableKeys.has(field.key) ||
            lockedByValidation.has(field.key);
          return (
            <Flex key={field.key} direction="column" gap="1">
              <Text size="1" weight="medium">
                {field.label}
                {locked ? (
                  <Text as="span" size="1" color="gray">
                    {" "}
                    (locked)
                  </Text>
                ) : null}
              </Text>
              {field.description ? (
                <Text size="1" color="gray">
                  {field.description}
                </Text>
              ) : null}
              <FieldEditor
                field={{ ...field, read_only: locked }}
                value={getNestedValue(
                  (draft.properties ?? {}),
                  field.key,
                )}
                onChange={(v) => updateProperty(field.key, v)}
              />
            </Flex>
          );
        })}
      </Flex>

      <Text size="1" weight="bold" mb="2" className={styles.sectionTitle}>
        Ports
      </Text>
      <Flex direction="column" gap="2" mb="4">
        {draft.ports.map((port) => (
          <Flex key={port.id} justify="between" className={styles.portRow}>
            <Text size="1">{port.name}</Text>
            <Text size="1" color="gray">
              {port.direction} · {port.datatype ?? "any"}
            </Text>
          </Flex>
        ))}
      </Flex>

      <Text size="1" weight="bold" mb="2" className={styles.sectionTitle}>
        Runtime (read-only)
      </Text>
      <Flex direction="column" gap="1" mb="4">
        <Text size="1" color="gray">
          Phase: {draft.execution?.phase ?? typeDef?.execution_semantics.phase ?? "—"}
        </Text>
        <Text size="1" color="gray">
          Trigger: {draft.execution?.trigger ?? typeDef?.execution_semantics.trigger ?? "—"}
        </Text>
        <Text size="1" color="gray">
          State: {draft.runtime_state ?? "unknown"}
        </Text>
      </Flex>

      <Flex gap="2" direction="column">
        {syncToProject ? (
          <Text size="1" color={syncPending ? "amber" : "gray"}>
            {syncPending
              ? "Saving to project…"
              : "Edits auto-save to main/app_config.h (~700ms after you stop typing)."}
          </Text>
        ) : (
          <Flex gap="2">
            <Button
              size="2"
              variant="solid"
              color={dirty ? "green" : "gray"}
              disabled={!dirty}
              onClick={handleApply}
            >
              Apply
            </Button>
            <Button
              size="2"
              variant="soft"
              color="gray"
              disabled={!dirty}
              onClick={handleRevert}
            >
              Revert
            </Button>
          </Flex>
        )}
        {syncToProject && dirty ? (
          <Button
            size="1"
            variant="soft"
            color="gray"
            onClick={handleRevert}
          >
            Revert
          </Button>
        ) : null}
      </Flex>
    </Box>
  );
};
