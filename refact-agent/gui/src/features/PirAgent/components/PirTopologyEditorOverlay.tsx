import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  applyDiagramPositions,
  extractDiagramPositions,
  type DiagramLayoutPositions,
} from "../layout/diagramLayoutPositions";
import type { FirmwareGraph } from "../types";
import { Box, Dialog, Flex, IconButton, Text } from "@radix-ui/themes";
import { Cross2Icon } from "@radix-ui/react-icons";
import { Zap, ZapOff } from "lucide-react";
import classNames from "classnames";
import { ReactFlowProvider } from "reactflow";

import type { PipelineStage } from "../../../hooks/useWorkflowStatus";
import type { usePirMaker } from "../hooks/usePirMaker";
import {
  isInspectorReadOnlyDiagramView,
  type GraphDiagramView,
} from "../layout/graphViewTypes";
import { GraphCanvas } from "./GraphCanvas";
import { GraphDiagramViewSelect } from "./GraphDiagramViewSelect";
import { MermaidDiagram } from "./MermaidDiagram";
import { NodeInspector } from "./NodeInspector";
import { TopologyApprovalCard } from "./TopologyApprovalCard";
import { TopologyErrorBoundary } from "./TopologyErrorBoundary";
import styles from "./PirTopologyChatBlock.module.css";

type PirMakerApi = ReturnType<typeof usePirMaker>;

export type PirTopologyEditorOverlayProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  pir: PirMakerApi;
  currentStage: PipelineStage;
};

export const PirTopologyEditorOverlay: React.FC<PirTopologyEditorOverlayProps> = ({
  open,
  onOpenChange,
  pir,
  currentStage,
}) => {
  const graph = pir.graph;
  const { loadViewDocument, viewDocuments } = pir;
  const [diagramView, setDiagramView] = useState<GraphDiagramView>("topology");
  const inspectorReadOnly = isInspectorReadOnlyDiagramView(diagramView);
  const [diagramPositions, setDiagramPositions] = useState<DiagramLayoutPositions>({});
  const { setSelectedNodeId } = pir;

  const [autoSave, setAutoSave] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("pirNodeAutoSave") !== "false";
    }
    return true;
  });

  const toggleAutoSave = useCallback(() => {
    setAutoSave((v) => {
      const next = !v;
      localStorage.setItem("pirNodeAutoSave", String(next));
      return next;
    });
  }, []);

  useEffect(() => {
    if (!open) {
      setDiagramView("topology");
      setDiagramPositions({});
    }
  }, [open]);

  useEffect(() => {
    if (!open || !graph) return;
    const fromGraph = extractDiagramPositions(graph);
    setDiagramPositions((prev) => ({
      ...prev,
      topology: { ...fromGraph, ...prev.topology },
    }));
  }, [open, graph, pir.pir?.revision]);

  useEffect(() => {
    if (inspectorReadOnly) setSelectedNodeId(null);
  }, [inspectorReadOnly, setSelectedNodeId]);

  useEffect(() => {
    if (!open || diagramView === "topology") return;
    void loadViewDocument(diagramView).catch(() => undefined);
  }, [open, diagramView, loadViewDocument]);

  const nodeConfidence = useMemo(() => {
    const map = new Map<string, number>();
    for (const n of pir.pir?.nodes ?? []) {
      map.set(n.id, n.confidence);
    }
    return map;
  }, [pir.pir?.nodes]);

  const sequenceDiagram = pir.pir?.diagrams?.sequence;
  const sequenceFromView = viewDocuments.sequence;
  const currentRevision = pir.pir?.revision;
  const currentGraphVersion = pir.pir?.graph_version;
  const isViewFresh = useCallback(
    (doc?: { revision: string; graphVersion?: number }) => {
      if (!doc || !currentRevision) return false;
      if (doc.revision !== currentRevision) return false;
      if (currentGraphVersion == null || doc.graphVersion == null) return true;
      return doc.graphVersion === currentGraphVersion;
    },
    [currentRevision, currentGraphVersion],
  );
  const freshSequenceView = isViewFresh(sequenceFromView)
    ? sequenceFromView
    : undefined;
  const sequenceMermaid =
    freshSequenceView?.mermaid ??
    sequenceDiagram?.mermaid ??
    pir.pir?.diagrams?.sequence_mermaid ??
    "";
  const sequenceTitle =
    freshSequenceView?.title ?? sequenceDiagram?.title ?? "Sequence Diagram";
  const sequenceGenerationError =
    freshSequenceView?.generationError ?? sequenceDiagram?.generation_error ?? null;

  const displayGraph = useMemo(() => {
    if (!graph) return null;
    const saved = diagramPositions[diagramView];
    if (diagramView === "topology") {
      return applyDiagramPositions(graph, saved);
    }
    const viewDoc = viewDocuments[diagramView];
    const viewGraph = isViewFresh(viewDoc) ? viewDoc?.graph : undefined;
    if (viewGraph) {
      return applyDiagramPositions(viewGraph, saved);
    }
    const d = pir.pir?.diagrams;
    if (!d) return null;
    if (diagramView === "hld" && d.hld_graph) {
      return applyDiagramPositions(d.hld_graph, saved);
    }
    if (diagramView === "ldd" && d.lld_graph) {
      return applyDiagramPositions(d.lld_graph, saved);
    }
    return null;
  }, [graph, pir.pir?.diagrams, viewDocuments, diagramView, diagramPositions, isViewFresh]);

  const onGraphLayoutChange = useCallback(
    (next: FirmwareGraph) => {
      const positions = extractDiagramPositions(next);
      setDiagramPositions((prev) => ({
        ...prev,
        [diagramView]: positions,
      }));
      if (diagramView === "topology") {
        pir.updateGraph(next);
      }
    },
    [diagramView, pir],
  );

  if (!graph) return null;

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Content className={styles.overlayContent} aria-describedby={undefined}>
        <Flex className={styles.overlayHeader} justify="between" align="start" gap="3">
          <Box style={{ flex: 1, minWidth: 0 }}>
            <TopologyApprovalCard
              pirStatus={pir.pirStatus}
              result={
                pir.pir && pir.validation
                  ? {
                      status: "ready",
                      pir: pir.pir,
                      graph,
                      validation: pir.validation,
                      diff: pir.diff,
                    }
                  : null
              }
              validation={pir.validation}
              loading={pir.loading || pir.pirStatus?.status === "analyzing"}
              onRefresh={() => void pir.runAnalyze(false, "user_refresh")}
            />
          </Box>
          <Flex align="center" gap="2" style={{ flexShrink: 0 }}>
            <button
              type="button"
              className={classNames(styles.autoSaveToggle, {
                [styles.autoSaveToggleActive]: autoSave,
              })}
              onClick={toggleAutoSave}
              title={
                autoSave
                  ? "Auto-save on — click to disable"
                  : "Auto-save off — click to save manually"
              }
              aria-pressed={autoSave}
            >
              {autoSave ? (
                <Zap size={12} strokeWidth={2.5} />
              ) : (
                <ZapOff size={12} strokeWidth={2.5} />
              )}
              <span>Auto-save</span>
            </button>
            <Dialog.Close>
              <IconButton variant="ghost" color="gray" aria-label="Close topology editor">
                <Cross2Icon />
              </IconButton>
            </Dialog.Close>
          </Flex>
        </Flex>

        {pir.error ? (
          <Box px="4" py="2">
            <Text size="1" color="red">
              {pir.error}
            </Text>
          </Box>
        ) : null}

        <Flex className={styles.overlayBody}>
          <Box className={styles.overlayDiagram}>
            <Box className={styles.diagramToolbar}>
              <GraphDiagramViewSelect
                value={diagramView}
                onChange={setDiagramView}
                disabled={pir.loading}
              />
            </Box>
            <TopologyErrorBoundary key={diagramView} label="graph">
              {diagramView === "sequence" ? (
                <MermaidDiagram
                  code={sequenceMermaid}
                  loading={pir.loading}
                  generationError={sequenceGenerationError}
                  title={sequenceTitle}
                  onRegenerate={() => void pir.runAnalyze(false, "user_refresh")}
                />
              ) : (
                <ReactFlowProvider>
                  {displayGraph ? (
                    <GraphCanvas
                      graph={displayGraph}
                      registry={pir.registry}
                      validationIssues={pir.validation?.issues ?? []}
                      selectedNodeId={pir.selectedNodeId}
                      onSelectNode={pir.setSelectedNodeId}
                      onGraphChange={onGraphLayoutChange}
                      diffNodeIds={pir.diffNodeIds}
                      nodeConfidence={nodeConfidence}
                      viewMode={
                        diagramView === "topology"
                          ? pir.loading
                            ? "readonly"
                            : "interactive"
                          : "diagram-layout"
                      }
                      diagramView={diagramView}
                    />
                  ) : (
                    <Box p="4">
                      <Text size="2" color="gray">
                        Diagram view is unavailable for the current revision. Refresh PIR to regenerate this view.
                      </Text>
                    </Box>
                  )}
                </ReactFlowProvider>
              )}
            </TopologyErrorBoundary>
          </Box>
          <Box className={styles.overlayInspector}>
            <TopologyErrorBoundary label="inspector">
              {inspectorReadOnly ? (
                <Box p="3">
                  <Text size="2" weight="bold" mb="2">
                    Inspector
                  </Text>
                  <Text size="1" color="gray">
                    Switch to <strong>Wiring topology</strong> to edit node variables.
                  </Text>
                </Box>
              ) : (
                <NodeInspector
                  node={pir.selectedNode}
                  typeDef={pir.selectedTypeDef}
                  pirNode={pir.selectedPirNode}
                  issues={pir.validation?.issues ?? []}
                  onApply={(id, updated) => void pir.applyNodeEdits(id, updated)}
                  onClose={() => pir.setSelectedNodeId(null)}
                  syncToProject={autoSave}
                />
              )}
            </TopologyErrorBoundary>
          </Box>
        </Flex>

        {currentStage === "COMPILING" && pir.pir?.approval.status !== "approved" ? (
          <Box px="4" py="2">
            <Text size="1" color="amber">
              Approve the topology before build when the diagram matches your intent.
            </Text>
          </Box>
        ) : null}
      </Dialog.Content>
    </Dialog.Root>
  );
};
