import React, { useState } from "react";
import { CheckIcon, Cross1Icon } from "@radix-ui/react-icons";
import { LightningBoltIcon } from "@radix-ui/react-icons";
import * as Collapsible from "@radix-ui/react-collapsible";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import { Text } from "@radix-ui/themes";
import { FaCode, FaListUl, FaMicrochip, FaDisplay } from "react-icons/fa6";
import styles from "./ProgressBar.module.css";
import { PipelineStage } from "../../hooks/useWorkflowStatus";
import type { ProgressEventDto } from "../../hooks/useProgress";

interface ProgressBarProps {
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
    case "partial_success":
      return "completed";
    case "failure":
      return "error";
    default:
      return "pending";
  }
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
  const currentIndex = STAGES.findIndex((s) => s.id === currentStage);
  const hasAnyEvents = events.length > 0;
  const currentNode = stageToNode(currentStage);

  const latestCurrentNodeEvent = [...events]
    .reverse()
    .find((evt) => evt.node === currentNode);

  const currentNodeIsRunning =
    latestCurrentNodeEvent?.status === "ongoing" ||
    latestCurrentNodeEvent?.status === "pending";
  const currentNodeHasError =
    latestCurrentNodeEvent?.status === "failure" ||
    latestCurrentNodeEvent?.status === "partial_success";
  const currentNodeCompleted =
    latestCurrentNodeEvent?.status === "success" ||
    latestCurrentNodeEvent?.status === "cached" ||
    latestCurrentNodeEvent?.status === "skipped";

  const getStepState = (index: number): string => {
    if (index < currentIndex) return "completed";
    if (index === currentIndex) {
      if (isDebugging) return "debugging";
      if (currentNodeHasError) return "error";
      if (currentNodeIsRunning || isStreaming) return "running";
      if (currentNodeCompleted) return "completed";
      if (!hasAnyEvents) return "pending";
      if (hasError) return "error";
      return "completed";
    }
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
    if (state === "error") {
      return (
        <div className={styles.errorCircle}>
          <Cross1Icon className={styles.errorIcon} />
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
          const isCurrentAndDebugging = index === currentIndex && isDebugging;
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
                        <Cross1Icon className={styles.statusIconError} />
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
