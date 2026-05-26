import React, { useState } from "react";
import { CheckIcon } from "@radix-ui/react-icons";
import { LightningBoltIcon } from "@radix-ui/react-icons";
import * as Collapsible from "@radix-ui/react-collapsible";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import { Text } from "@radix-ui/themes";
import { FaCode, FaListUl, FaMicrochip, FaDisplay } from "react-icons/fa6";
import styles from "./ProgressBar.module.css";
import { PipelineStage } from "../../hooks/useWorkflowStatus";
import type { ProgressEventDto } from "../../hooks/useProgress";

export interface ProgressBarProps {
  currentStage: PipelineStage;
  isStreaming?: boolean;
  hasError?: boolean;
  isDebugging?: boolean;
  debugIteration?: number;
  events?: ProgressEventDto[];
}

const STAGES: Array<{ id: PipelineStage; label: string; debugLabel: string }> = [
  { id: "PLANNING", label: "planning", debugLabel: "planning" },
  { id: "GENERATION", label: "coding", debugLabel: "coding" },
  { id: "COMPILING", label: "building", debugLabel: "debugging" },
  { id: "FLASH", label: "flashing", debugLabel: "flashing" },
  { id: "MONITORING", label: "monitoring", debugLabel: "debugging" },
];

function stageToNode(stage: PipelineStage): ProgressEventDto["node"] {
  switch (stage) {
    case "PLANNING":
      return "planning";
    case "GENERATION":
      return "generation";
    case "COMPILING":
      return "compiling";
    case "FLASH":
      return "flashing";
    case "MONITORING":
      return "monitoring";
    default:
      return "planning";
  }
}

function getEventStatusDisplay(status: string): "running" | "completed" | "error" | "pending" {
  switch (status) {
    case "ongoing":
      return "running";
    case "success":
    case "cached":
    case "skipped":
      return "completed";
    // partial_success: tool ran but had issues — show as error, not silent completion
    case "partial_success":
    case "failure":
      return "error";
    default:
      return "pending";
  }
}

const NODE_ORDER: ProgressEventDto["node"][] = [
  "planning",
  "generation",
  "compiling",
  "flashing",
  "monitoring",
];

function nodeOrdinal(node: ProgressEventDto["node"]): number {
  const idx = NODE_ORDER.indexOf(node);
  return idx >= 0 ? idx : 0;
}

function latestEventForNode(
  events: ProgressEventDto[],
  node: ProgressEventDto["node"],
): ProgressEventDto | undefined {
  return [...events].reverse().find((evt) => evt.node === node);
}

/**
 * Determine which step index should be treated as the "current" one.
 *
 * Priority:
 * 1. The most recent currently-running event's node (live activity).
 * 2. currentStage as reported by the backend (authoritative after each poll).
 *
 * We intentionally do NOT use "max historical ordinal" — that causes the
 * indicator to stick at the highest stage ever reached (e.g. monitoring)
 * even when a later detect/flash is running at a lower stage.
 */
function deriveHighlightIndex(
  events: ProgressEventDto[],
  currentStage: PipelineStage,
): number {
  // Walk events newest-first and find the first still-running one.
  for (let i = events.length - 1; i >= 0; i--) {
    if (getEventStatusDisplay(events[i].status) === "running") {
      const activeOrdinal = nodeOrdinal(events[i].node);
      const stageIdx = STAGES.findIndex((s) => s.id === currentStage);
      // Take the larger of the two so the indicator never goes below what the
      // backend has authoritatively confirmed as current.
      return Math.max(activeOrdinal, stageIdx >= 0 ? stageIdx : 0);
    }
  }
  // No live event — trust the backend stage entirely.
  const stageIdx = STAGES.findIndex((s) => s.id === currentStage);
  return stageIdx >= 0 ? stageIdx : 0;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  currentStage,
  isStreaming = false,
  hasError = false,
  isDebugging = false,
  debugIteration = 0,
  events = [],
}) => {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const highlightIndex = deriveHighlightIndex(events, currentStage);
  const hasAnyEvents = events.length > 0;

  const getStepState = (index: number): string => {
    const stageId = STAGES[index]?.id;
    if (!stageId) return "pending";

    const node = stageToNode(stageId);

    // ── Highest priority: any running event for THIS exact node ──────────
    // Fires regardless of highlightIndex so "detect" at flashing lights the
    // flashing step even if monitoring was previously completed.
    const hasRunning = events.some(
      (e) => e.node === node && getEventStatusDisplay(e.status) === "running",
    );
    if (hasRunning) return "running";

    // ── A node is completed ONLY when it has actual success events ────────
    // We never treat "index < highlightIndex" as automatically completed —
    // that was causing planning/coding/building to light up green when the
    // user jumps straight to detect/flashing without going through them.
    const hasCompleted = events.some(
      (e) => e.node === node && getEventStatusDisplay(e.status) === "completed",
    );
    if (hasCompleted) return "completed";

    // ── Current highlighted stage (from backend current_node) ────────────
    if (index === highlightIndex) {
      if (isDebugging) return "debugging";
      // Polling lag fallback: show spinner while the chat is streaming but
      // no event has arrived yet for this node.
      if (isStreaming) return "running";
      return "pending";
    }

    // ── Future stage (or past stage with no recorded events) ─────────────
    return "pending";
  };

  const renderNode = (stageId: PipelineStage, state: string) => {
    if (state === "completed") {
      const CompletedIcon =
        stageId === "PLANNING"
          ? FaListUl
          : stageId === "GENERATION"
            ? FaCode
            : stageId === "COMPILING"
              ? FaMicrochip
              : stageId === "MONITORING"
                ? FaDisplay
                : LightningBoltIcon; // FLASH
      return (
        <div className={styles.checkmarkCircle}>
          <CompletedIcon className={styles.checkmark} />
        </div>
      );
    }
    if (state === "debugging") {
      return (
        <div className={styles.debugCircle}>
          <span className={styles.debugIcon}>&#9881;</span>
        </div>
      );
    }
    if (state === "running") {
      return (
        <div className={styles.loadingRing}>
          <div className={styles.loadingSpinner} />
          <div className={styles.hollowCircle} />
        </div>
      );
    }
    return <div className={styles.hollowCircle} />;
  };

  return (
    <div className={styles.container}>
      <div className={styles.track}>
        {STAGES.map((stage, index) => {
          const state = getStepState(index);
          const isCurrentAndDebugging = index === highlightIndex && isDebugging;
          const displayLabel = isCurrentAndDebugging ? stage.debugLabel : stage.label;
          return (
            <React.Fragment key={stage.id}>
              <div
                className={`${styles.step} ${styles[`stage_${stage.id.toLowerCase()}`]} ${styles[state]}`}
                title={displayLabel}
              >
                <div className={styles.dotWrapper}>{renderNode(stage.id, state)}</div>
                <span className={styles.label}>
                  {displayLabel}
                  {isCurrentAndDebugging && debugIteration > 0 && (
                    <span className={styles.iterationBadge}>#{debugIteration}</span>
                  )}
                </span>
              </div>
            </React.Fragment>
          );
        })}
      </div>

      {events.length > 0 && (
        <Collapsible.Root open={detailsOpen} onOpenChange={setDetailsOpen}>
          <Collapsible.Trigger asChild>
            <button
              type="button"
              className={styles.toolCallsTrigger}
              aria-expanded={detailsOpen}
            >
              <ChevronDownIcon
                className={styles.chevron}
                data-open={detailsOpen}
              />
              <Text size="1" color="gray">
                {events.length} tool call{events.length !== 1 ? "s" : ""}
              </Text>
            </button>
          </Collapsible.Trigger>
          <Collapsible.Content>
            <div className={styles.toolCallsList}>
              <div className={styles.toolCallsHeader}>
                <span>Tool</span>
                <span>Operation</span>
                <span>Status</span>
              </div>
              {events.map((evt) => {
                const displayStatus = getEventStatusDisplay(evt.status);
                return (
                  <div
                    key={evt.id}
                    className={`${styles.toolCallRow} ${styles[`row_${displayStatus}`]}`}
                  >
                    <span className={styles.toolName}>{evt.tool_name}</span>
                    <span className={styles.operation}>{evt.operation}</span>
                    <span className={styles.statusCell}>
                      {displayStatus === "running" && (
                        <span className={styles.spinner} />
                      )}
                      {displayStatus === "completed" && (
                        <CheckIcon className={styles.statusIcon} />
                      )}
                      {displayStatus === "error" && (
                        <span className={styles.statusIconError}>✕</span>
                      )}
                      {displayStatus === "pending" && (
                        <span className={styles.pendingDot} />
                      )}
                      <span className={styles.statusLabel}>
                        {displayStatus}
                      </span>
                    </span>
                  </div>
                );
              })}
            </div>
          </Collapsible.Content>
        </Collapsible.Root>
      )}
    </div>
  );
};
