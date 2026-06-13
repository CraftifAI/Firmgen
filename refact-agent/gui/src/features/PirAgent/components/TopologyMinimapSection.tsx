import React, { useState, useCallback, useMemo } from "react";
import { Button, Flex, Text } from "@radix-ui/themes";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import { Network, Zap, ZapOff } from "lucide-react";
import { ReactFlowProvider } from "reactflow";
import classNames from "classnames";

import { isAssistantMessage } from "../../../services/refact";
import { useAppSelector } from "../../../hooks";
import {
  selectChatId,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectThread,
} from "../../Chat/Thread/selectors";
import { usePirMaker } from "../hooks/usePirMaker";
import type { RootState } from "../../../app/store";
import { GraphCanvas } from "./GraphCanvas";
import { ignoreNodeSelection } from "../utils/noop";
import { PirTopologyEditorOverlay } from "./PirTopologyEditorOverlay";
import type { PipelineStage } from "../../../hooks/useWorkflowStatus";
import styles from "./TopologyMinimapSection.module.css";

export type TopologyMinimapSectionProps = {
  /** Path from backend progress events — used as a fallback when the thread has none. */
  projectPath: string | null;
  currentStage: PipelineStage;
};

export const TopologyMinimapSection: React.FC<TopologyMinimapSectionProps> = ({
  projectPath: progressProjectPath,
  currentStage,
}) => {
  const [open, setOpen] = useState(false);
  const [overlayOpen, setOverlayOpen] = useState(false);
  const [autoUpdate, setAutoUpdate] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("pirAutoUpdate") !== "false";
    }
    return true;
  });

  const toggleAutoUpdate = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setAutoUpdate((v) => {
      const next = !v;
      localStorage.setItem("pirAutoUpdate", String(next));
      return next;
    });
  }, []);

  const chatId = useAppSelector(selectChatId);
  const thread = useAppSelector(selectThread);
  const messages = useAppSelector(selectMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const wsProjects = useAppSelector((s: RootState) => s.workspaceProjects.projects);
  const activeProjectId = useAppSelector((s: RootState) => s.workspaceProjects.activeProjectId);

  // Build project path from the richest available source, in priority order.
  const projectPath = useMemo(() => {
    // 1. Thread carries its own path (set at chat creation or via setChatProject)
    const fromThread = thread.esp32_projects_path?.trim();
    if (fromThread) return fromThread;

    // 2. Thread has a project_id — look up the path from workspaceProjects
    if (thread.project_id) {
      const byId = wsProjects.find((p) => p.id === thread.project_id);
      const fromId = byId?.esp32_projects_path.trim();
      if (fromId) return fromId;
    }

    // 3. Sidebar has an actively selected project
    if (activeProjectId) {
      const active = wsProjects.find((p) => p.id === activeProjectId);
      const fromActive = active?.esp32_projects_path.trim();
      if (fromActive) return fromActive;
    }

    // 4. Backend progress event (only live during an active build/flash)
    const fromProgress = progressProjectPath?.trim();
    if (fromProgress) return fromProgress;

    return null;
  }, [
    thread.esp32_projects_path,
    thread.project_id,
    wsProjects,
    activeProjectId,
    progressProjectPath,
  ]);

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

  const pir = usePirMaker({
    chatId: chatId ?? "",
    projectPath: projectPath ?? undefined,
    pollMs: 12000,
    enabled: Boolean(chatId && projectPath),
    isAgentStreaming: isStreaming,
    reanalyzeWhenAgentIdle: autoUpdate,
    agentTurnId: lastAgentTurnId,
    chatContext: pirChatContext,
    enableLiveWatch: false,
    skipMountAnalyze: true,
    hydrateOnMount: true,
    codegenReady: true,
  });

  const graph = pir.graph;
  const hasGraph = Boolean(graph && graph.nodes.length > 0);
  const nodeCount = graph?.nodes.length ?? 0;
  const isAnalyzing = pir.loading || pir.pirStatus?.status === "analyzing";
  const headline =
    pir.pir?.summary?.headline ?? pir.pirStatus?.summary_headline ?? null;

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

  // Only hide if there's no chat or no project — no codegen gate.
  if (!chatId || !projectPath) {
    console.debug("[TopologyMinimap] hidden — chatId:", chatId, "projectPath:", projectPath,
      "thread.esp32:", thread.esp32_projects_path, "thread.project_id:", thread.project_id,
      "activeProjectId:", activeProjectId, "wsProjects:", wsProjects.length);
    return null;
  }

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
          <button
            type="button"
            className={classNames(styles.autoToggle, {
              [styles.autoToggleActive]: autoUpdate,
            })}
            onClick={toggleAutoUpdate}
            title={
              autoUpdate
                ? "Auto-update on — click to disable"
                : "Auto-update off — click to enable"
            }
            aria-pressed={autoUpdate}
          >
            {autoUpdate ? (
              <Zap size={11} strokeWidth={2.5} />
            ) : (
              <ZapOff size={11} strokeWidth={2.5} />
            )}
          </button>
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
            {headline && !isAnalyzing ? (
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
            ) : pir.error ? (
              <div className={styles.empty}>
                <Network
                  size={18}
                  strokeWidth={1.25}
                  className={styles.emptyIcon}
                />
                <span className={styles.emptyText}>
                  Analyze failed: {pir.error}
                </span>
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
