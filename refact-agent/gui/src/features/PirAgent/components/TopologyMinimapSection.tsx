import React, { useState, useCallback, useMemo } from "react";
import { Button, Flex, Text } from "@radix-ui/themes";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import { Network } from "lucide-react";
import { ReactFlowProvider } from "reactflow";
import classNames from "classnames";

import { isAssistantMessage } from "../../../services/refact";
import { useAppSelector } from "../../../hooks";
import {
  selectChatId,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
} from "../../Chat/Thread/selectors";
import { usePirCodegenReady } from "../hooks/usePirCodegenReady";
import { usePirChatAnchor } from "../hooks/usePirChatAnchor";
import { usePirMaker } from "../hooks/usePirMaker";
import { GraphCanvas } from "./GraphCanvas";
import { ignoreNodeSelection } from "../utils/noop";
import { PirTopologyEditorOverlay } from "./PirTopologyEditorOverlay";
import type { PipelineStage } from "../../../hooks/useWorkflowStatus";
import styles from "./TopologyMinimapSection.module.css";

export type TopologyMinimapSectionProps = {
  projectPath: string | null;
  currentStage: PipelineStage;
};

export const TopologyMinimapSection: React.FC<TopologyMinimapSectionProps> = ({
  projectPath,
  currentStage,
}) => {
  const [open, setOpen] = useState(false);
  const [overlayOpen, setOverlayOpen] = useState(false);

  const chatId = useAppSelector(selectChatId);
  const messages = useAppSelector(selectMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);

  const agentIsWorking = isStreaming || isWaiting;

  const lastAgentTurnId = useMemo(() => {
    if (agentIsWorking) return null;
    let lastIndex = -1;
    messages.forEach((m, i) => {
      if (isAssistantMessage(m)) lastIndex = i;
    });
    return lastIndex >= 0 ? `assistant-${lastIndex}` : null;
  }, [messages, agentIsWorking]);

  const pirChatContext = useMemo(() => {
    return messages
      .filter((m) => m.role === "user" || m.role === "assistant")
      .slice(-6)
      .map((m) => {
        if (typeof m.content === "string") {
          return `${m.role.toUpperCase()}: ${m.content.slice(0, 400)}`;
        }
        return null;
      })
      .filter(Boolean)
      .join("\n");
  }, [messages]);

  const { ready: codegenReady, checking: codegenChecking } = usePirCodegenReady({
    chatId: chatId ?? null,
    projectPath,
    agentTurnId: lastAgentTurnId,
    agentIsWorking,
  });

  const {
    anchorTurnId,
    effectiveProjectPath: pirProjectPath,
    showBlock,
  } = usePirChatAnchor({
    chatId: chatId ?? null,
    projectPath,
    codegenReady,
    agentTurnId: lastAgentTurnId,
    agentIsWorking,
  });

  const pir = usePirMaker({
    chatId: chatId ?? "",
    projectPath: pirProjectPath,
    pollMs: 12000,
    enabled: showBlock && Boolean(chatId && pirProjectPath),
    isAgentStreaming: isStreaming,
    reanalyzeWhenAgentIdle: true,
    agentTurnId: anchorTurnId,
    chatContext: pirChatContext,
    enableLiveWatch: false,
    skipMountAnalyze: true,
    hydrateOnMount: true,
    codegenReady: codegenReady || Boolean(anchorTurnId),
  });

  const graph = pir.graph;
  const hasGraph = Boolean(graph && graph.nodes.length > 0);
  const nodeCount = graph?.nodes.length ?? 0;
  const isAnalyzing = pir.loading || pir.pirStatus?.status === "analyzing";
  const headline =
    pir.pir?.summary?.headline ?? pir.pirStatus?.summary_headline ?? null;

  const validationErrors =
    pir.validation?.issues.filter((i) => i.severity === "error").length ?? 0;
  const validationWarnings =
    pir.validation?.issues.filter((i) => i.severity === "warning").length ?? 0;

  const nodeConfidence = useMemo(() => {
    const map = new Map<string, number>();
    for (const n of pir.pir?.nodes ?? []) {
      map.set(n.id, n.confidence);
    }
    return map;
  }, [pir.pir?.nodes]);

  const openEditor = useCallback(() => {
    if (hasGraph) setOverlayOpen(true);
  }, [hasGraph]);

  if (!showBlock || !chatId || !pirProjectPath) return null;

  return (
    <>
      <div className={styles.root}>
        {/* ── Section header ── */}
        <button
          type="button"
          className={classNames(styles.trigger, { [styles.triggerOpen]: open })}
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
        >
          <span className={styles.iconWrap}>
            <Network size={11} strokeWidth={2.5} />
          </span>
          <span className={styles.label}>Topology</span>
          {isAnalyzing ? (
            <span className={styles.spinner} />
          ) : hasGraph ? (
            <span className={styles.badge}>{nodeCount}</span>
          ) : null}
          <ChevronDownIcon
            width={12}
            height={12}
            className={classNames(styles.chevron, { [styles.chevronOpen]: open })}
          />
        </button>

        {/* ── Expandable body ── */}
        {open && (
          <div className={styles.content}>
            {/* subtitle */}
            {codegenChecking ? (
              <p className={styles.subtext}>Waiting for project code…</p>
            ) : headline && !isAnalyzing ? (
              <p className={styles.headline}>{headline}</p>
            ) : null}

            {/* graph preview */}
            {hasGraph && graph ? (
              <div
                className={styles.canvas}
                onClick={openEditor}
                role="button"
                tabIndex={0}
                aria-label="Topology — click to open full editor"
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    openEditor();
                  }
                }}
              >
                <div className={styles.canvasInner}>
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
                </div>
                <div className={styles.hoverOverlay} aria-hidden>
                  <span className={styles.hoverLabel}>Open editor ↗</span>
                </div>
              </div>
            ) : isAnalyzing ? (
              <div className={styles.analyzing}>
                <span className={styles.spinner} />
                <span className={styles.analyzingText}>Building topology…</span>
              </div>
            ) : (
              <div className={styles.empty}>
                <Network
                  size={18}
                  strokeWidth={1.25}
                  className={styles.emptyIcon}
                />
                <span className={styles.emptyText}>No topology yet</span>
              </div>
            )}

            {/* validation */}
            {(validationErrors > 0 || validationWarnings > 0) && (
              <Flex gap="2" mt="1" wrap="wrap">
                {validationErrors > 0 && (
                  <Text size="1" color="red">
                    {validationErrors} error{validationErrors === 1 ? "" : "s"}
                  </Text>
                )}
                {validationWarnings > 0 && (
                  <Text size="1" color="amber">
                    {validationWarnings} warning
                    {validationWarnings === 1 ? "" : "s"}
                  </Text>
                )}
              </Flex>
            )}

            {/* footer */}
            <Flex mt="2" gap="2" align="center">
              <Button
                size="1"
                variant="ghost"
                className={styles.actionBtn}
                onClick={() => void pir.runAnalyze(true, "user_refresh")}
                disabled={isAnalyzing}
              >
                Refresh
              </Button>
              {hasGraph && (
                <Button
                  size="1"
                  variant="ghost"
                  className={styles.actionBtn}
                  onClick={openEditor}
                >
                  Open editor
                </Button>
              )}
            </Flex>
          </div>
        )}
      </div>

      {hasGraph && (
        <PirTopologyEditorOverlay
          open={overlayOpen}
          onOpenChange={setOverlayOpen}
          pir={pir}
          currentStage={currentStage}
        />
      )}
    </>
  );
};
