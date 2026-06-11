import React, { useCallback, useMemo, useState } from "react";
import { Box, Container, Flex, Text } from "@radix-ui/themes";
import * as Collapsible from "@radix-ui/react-collapsible";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import { Network } from "lucide-react";
import { ReactFlowProvider } from "reactflow";
import classNames from "classnames";

import type { PipelineStage } from "../../../hooks/useWorkflowStatus";
import { usePirMaker } from "../hooks/usePirMaker";
import { GraphCanvas } from "./GraphCanvas";
import { PirAnalyzedFilesList } from "./PirAnalyzedFilesList";
import { ignoreNodeSelection } from "../utils/noop";
import { PirTopologyEditorOverlay } from "./PirTopologyEditorOverlay";
import { TopologyApprovalCard } from "./TopologyApprovalCard";
import styles from "./PirTopologyChatBlock.module.css";

export type PirTopologyChatBlockProps = {
  chatId: string;
  projectPath: string;
  currentStage: PipelineStage;
  isAgentStreaming: boolean;
  agentTurnId?: string | null;
  chatContext?: string | null;
  codegenReady?: boolean;
  codegenChecking?: boolean;
};

/**
 * Inline chat disclosure block (same pattern as File changes / Diff).
 * Renders after the latest assistant turn; preview in-thread, fullscreen editor on demand.
 */
export const PirTopologyChatBlock: React.FC<PirTopologyChatBlockProps> = React.memo(({
  chatId,
  projectPath,
  currentStage,
  isAgentStreaming,
  agentTurnId = null,
  chatContext = null,
  codegenReady = false,
  codegenChecking = false,
}) => {
  const path = projectPath.trim();
  const [open, setOpen] = useState(true);
  const [overlayOpen, setOverlayOpen] = useState(false);

  const pir = usePirMaker({
    chatId,
    projectPath: path,
    pollMs: 10000,
    enabled: Boolean(path),
    isAgentStreaming,
    reanalyzeWhenAgentIdle: true,
    agentTurnId,
    chatContext,
    enableLiveWatch: false,
    skipMountAnalyze: true,
    hydrateOnMount: true,
    codegenReady: codegenReady || Boolean(agentTurnId),
  });

  const graph = pir.graph;
  const isAnalyzing = pir.loading || pir.pirStatus?.status === "analyzing";
  const hasGraph = Boolean(graph && graph.nodes.length > 0);
  const nodeCount = graph?.nodes.length ?? 0;

  const nodeConfidence = useMemo(() => {
    const map = new Map<string, number>();
    for (const n of pir.pir?.nodes ?? []) {
      map.set(n.id, n.confidence);
    }
    return map;
  }, [pir.pir?.nodes]);

  const headline =
    pir.pir?.summary?.headline ??
    pir.pirStatus?.summary_headline ??
    "Project wiring and configuration";

  const approval =
    pir.pir?.approval.status ?? pir.pirStatus?.approval_status ?? "pending";

  const needsApprovalHint =
    hasGraph &&
    !isAnalyzing &&
    (approval === "pending" || approval === "stale");

  const analyzedFiles = pir.pir?.provenance.analyzed_files ?? [];

  const openEditor = useCallback(() => {
    if (hasGraph) setOverlayOpen(true);
  }, [hasGraph]);

  if (!path) return null;

  return (
    <Container>
      <Flex direction="column" py="3" gap="2">
        <Collapsible.Root open={open} onOpenChange={setOpen} className={styles.disclosure}>
          <Collapsible.Trigger asChild>
            <button
              type="button"
              className={classNames(styles.trigger, { [styles.triggerOpen]: open })}
              aria-expanded={open}
            >
              <span className={styles.iconWrap}>
                <Network size={16} strokeWidth={2} />
              </span>
              <span className={styles.body}>
                <span className={styles.label}>Firmware topology</span>
                <span className={styles.subtitle}>
                  {codegenChecking
                    ? "Waiting for project code from main agent…"
                    : isAnalyzing
                      ? "Building topology from app_config.h…"
                      : headline}
                </span>
              </span>
              <span className={styles.meta}>
                {isAnalyzing
                  ? "Analyzing…"
                  : hasGraph
                    ? `${nodeCount} node${nodeCount === 1 ? "" : "s"} · ${approval}`
                    : approval}
              </span>
              <ChevronDownIcon
                className={classNames(styles.chevron, { [styles.chevronOpen]: open })}
              />
            </button>
          </Collapsible.Trigger>

          <Collapsible.Content className={styles.content}>
            {pir.error ? (
              <Text size="1" color="red" mb="2">
                {pir.error}
              </Text>
            ) : null}

            {isAnalyzing && !hasGraph ? (
              <Flex className={styles.analyzingBox}>
                <Text size="1" color="gray">
                  Building project graph…
                </Text>
              </Flex>
            ) : null}

            {needsApprovalHint ? (
              <Text size="1" color="amber" mb="2">
                Approve topology before confirming build with the agent.
              </Text>
            ) : null}

            {hasGraph && graph ? (
              <>
                <Box
                  className={styles.previewPane}
                  onDoubleClick={openEditor}
                  role="button"
                  tabIndex={0}
                  aria-label="Topology preview. Double-click to open editor."
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      openEditor();
                    }
                  }}
                >
                  <ReactFlowProvider>
                    <GraphCanvas
                      graph={graph}
                      registry={pir.registry}
                      validationIssues={pir.validation?.issues ?? []}
                      selectedNodeId={null}
                      onSelectNode={ignoreNodeSelection}
                      diffNodeIds={pir.diffNodeIds}
                      nodeConfidence={nodeConfidence}
                      viewMode="preview"
                    />
                  </ReactFlowProvider>
                </Box>
                <button type="button" className={styles.openLink} onClick={openEditor}>
                  Click to view and edit graph
                </button>
              </>
            ) : null}

            {!hasGraph && !isAnalyzing && !pir.error ? (
              <Flex className={styles.analyzingBox} direction="column" gap="1">
                <Text size="1" color="gray">
                  No topology nodes yet.
                </Text>
                <Text size="1" color="gray">
                  Appears after firmware is generated in this project.
                </Text>
              </Flex>
            ) : null}

            {analyzedFiles.length > 0 ? (
              <Box mt="2">
                <PirAnalyzedFilesList files={analyzedFiles} compact />
              </Box>
            ) : null}

            {hasGraph ? (
              <Box mt="2">
                <TopologyApprovalCard
                  pirStatus={pir.pirStatus}
                  result={pir.result}
                  validation={pir.validation}
                  loading={isAnalyzing}
                  onApprove={() => void pir.approveTopology()}
                  onRefresh={() => void pir.runAnalyze(true, "user_refresh")}
                  onOpenDiagram={openEditor}
                />
              </Box>
            ) : null}
          </Collapsible.Content>
        </Collapsible.Root>

        {hasGraph ? (
          <PirTopologyEditorOverlay
            open={overlayOpen}
            onOpenChange={setOverlayOpen}
            pir={pir}
            currentStage={currentStage}
          />
        ) : null}
      </Flex>
    </Container>
  );
});
PirTopologyChatBlock.displayName = "PirTopologyChatBlock";
