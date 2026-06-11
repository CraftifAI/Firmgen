import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useConfig } from "../../../hooks";
import {
  pirAnalyze,
  pirApplyNodePatch,
  pirApprove,
  pirDocument,
  pirFetchNodeRegistry,
  pirGraphViewDocument,
  pirStatus as fetchPirStatus,
  pirUnwatch,
  pirWatch,
} from "../../../services/refact/pirMaker";
import type { PirGraphView } from "../../../services/refact/pirMaker";
import type {
  FirmwareGraph,
  FirmwareNode,
  GraphOrientation,
  NodeTypeDef,
} from "../types";
import type { GraphDiagramView } from "../layout/graphViewTypes";
import type {
  GraphLifecycleMode,
  PirAnalyzeResult,
  PirStatus,
} from "../pirTypes";
import {
  graphHasLayoutCoordinates,
  layoutGraph,
  layoutGraphPreservingPositions,
  mergeGraphVisualPositions,
} from "../layout/applyLayout";
import { isViewDocumentCurrent } from "./viewDocumentFreshness";

function propsAsRecord(props: unknown): Record<string, unknown> {
  if (props && typeof props === "object" && !Array.isArray(props)) {
    return props as Record<string, unknown>;
  }
  return {};
}

/** Survives React remounts — prevents duplicate agent_turn analyzes for the same chat turn. */
const pirTurnAnalyzeDedupe = new Map<string, string>();

function jsonStable(v: unknown): string {
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}

type ViewDocument = {
  revision: string;
  graphVersion?: number;
  graph?: FirmwareGraph;
  mermaid?: string;
  title?: string;
  generationError?: string;
};

type ViewDocumentCache = Partial<Record<GraphDiagramView, ViewDocument>>;

function toGraphViewApiName(view: GraphDiagramView): PirGraphView {
  switch (view) {
    case "topology":
      return "wiring";
    case "hld":
      return "hld";
    case "ldd":
      return "lld";
    case "sequence":
      return "sequence";
  }
}

export type UsePirMakerOptions = {
  chatId: string;
  projectPath?: string;
  pollMs?: number;
  enabled?: boolean;
  /** Re-analyze once after the agent completes a new turn (requires agentTurnId). */
  reanalyzeWhenAgentIdle?: boolean;
  /** Id of the latest assistant message — used to dedupe agent-idle triggers. */
  agentTurnId?: string | null;
  /** Recent main-chat user text for PIR gap inference. */
  chatContext?: string | null;
  isAgentStreaming?: boolean;
  /** Register filesystem watcher for live sync (off by default — prompt-driven updates only). */
  enableLiveWatch?: boolean;
  /** When true, skip mount-time analyze — PIR runs only on agent_idle / user refresh. */
  skipMountAnalyze?: boolean;
  /** Load cached PIR document on mount without triggering analyze (chat reload). */
  hydrateOnMount?: boolean;
  /** Parent gate: main agent produced app_config.h / main sources for this turn. */
  codegenReady?: boolean;
};

export function usePirMaker({
  chatId,
  projectPath,
  pollMs = 10000,
  enabled = true,
  reanalyzeWhenAgentIdle = false,
  agentTurnId = null,
  chatContext = null,
  isAgentStreaming = false,
  enableLiveWatch = false,
  skipMountAnalyze = false,
  hydrateOnMount = false,
  codegenReady = true,
}: UsePirMakerOptions) {
  const config = useConfig();
  const port = config.lspPort;

  const [result, setResult] = useState<PirAnalyzeResult | null>(null);
  const [registry, setRegistry] = useState<NodeTypeDef[]>([]);
  const [pirStatus, setPirStatus] = useState<PirStatus | null>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [orientation, setOrientation] = useState<GraphOrientation>("horizontal");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lifecycleMode, setLifecycleMode] =
    useState<GraphLifecycleMode>("idle");
  const [viewDocuments, setViewDocuments] = useState<ViewDocumentCache>({});

  const chatContextRef = useRef<string | null>(null);
  chatContextRef.current = chatContext?.trim() ?? null;

  const registryRef = useRef<NodeTypeDef[]>([]);
  const orientationRef = useRef(orientation);
  orientationRef.current = orientation;

  const watchStarted = useRef(false);
  const wasAgentStreaming = useRef(false);
  const pollStatusRef = useRef<string | undefined>(undefined);
  const graphRevisionRef = useRef<string | null>(null);
  const graphVersionRef = useRef<number | null>(null);
  const mountedKeyRef = useRef<string | null>(null);
  const lastAnalyzedTurnRef = useRef<string | null>(null);
  const agentIdleTimerRef = useRef<number | null>(null);
  const analyzeInFlightRef = useRef(false);
  const resultRef = useRef<PirAnalyzeResult | null>(null);
  resultRef.current = result;
  const viewDocumentsRef = useRef<ViewDocumentCache>({});
  viewDocumentsRef.current = viewDocuments;

  const graph = result?.graph ?? null;
  const pir = result?.pir ?? null;
  const validation = result?.validation ?? null;

  const applyGraphSnapshot = useCallback(
    (
      doc: PirAnalyzeResult,
      source: string,
      opts?: { clearSelection?: boolean; preservePositions?: boolean },
    ) => {
      const prevRevision = graphRevisionRef.current;
      const prevGraphVersion = graphVersionRef.current;
      const nextRevision = doc.pir.revision;
      const nextGraphVersion = doc.pir.graph_version;
      const revisionUnchanged =
        prevRevision !== null && prevRevision === nextRevision;
      const graphVersionUnchanged =
        prevGraphVersion !== null && prevGraphVersion === nextGraphVersion;
      const isPatchResponse = source === "patch" || source.startsWith("patch:");

      if (
        revisionUnchanged &&
        graphVersionUnchanged &&
        resultRef.current &&
        !isPatchResponse
      ) {
        setLifecycleMode("stable");
        return false;
      }

      const shouldPreserveLayout =
        isPatchResponse || opts?.preservePositions === true;

      let nextGraph = doc.graph;
      if (shouldPreserveLayout && resultRef.current?.graph) {
        nextGraph = mergeGraphVisualPositions(
          doc.graph,
          resultRef.current.graph,
        );
      }

      const laidOut = shouldPreserveLayout
        ? layoutGraphPreservingPositions(nextGraph, orientationRef.current)
        : graphHasLayoutCoordinates(nextGraph)
          ? layoutGraphPreservingPositions(nextGraph, orientationRef.current)
          : layoutGraph(nextGraph, orientationRef.current);
      graphRevisionRef.current = nextRevision;
      graphVersionRef.current = nextGraphVersion;
      const nextViewDocuments: ViewDocumentCache = {
        topology: {
          revision: nextRevision,
          graphVersion: nextGraphVersion,
          graph: laidOut,
        },
      };
      const diagrams = doc.pir.diagrams;
      if (diagrams?.hld_graph) {
        nextViewDocuments.hld = {
          revision: nextRevision,
          graphVersion: nextGraphVersion,
          graph: diagrams.hld_graph,
          mermaid: diagrams.hld?.mermaid ?? diagrams.hld_mermaid,
          title: diagrams.hld?.title ?? "High-Level Design",
        };
      }
      if (diagrams?.lld_graph) {
        nextViewDocuments.ldd = {
          revision: nextRevision,
          graphVersion: nextGraphVersion,
          graph: diagrams.lld_graph,
          mermaid: diagrams.lld?.mermaid ?? diagrams.lld_mermaid,
          title: diagrams.lld?.title ?? "Low-Level Design",
        };
      }
      nextViewDocuments.sequence = {
        revision: nextRevision,
        graphVersion: nextGraphVersion,
        graph: diagrams?.sequence_graph,
        mermaid: diagrams?.sequence?.mermaid ?? diagrams?.sequence_mermaid,
        title: diagrams?.sequence?.title ?? "Sequence Diagram",
        generationError: diagrams?.sequence?.generation_error,
      };
      setResult({ ...doc, graph: laidOut });
      setViewDocuments(nextViewDocuments);
      if (opts?.clearSelection) {
        setSelectedNodeId(null);
      }
      setLifecycleMode("stable");
      return true;
    },
    [],
  );

  const runAnalyze = useCallback(
    async (incremental = false, triggeredBy?: string) => {
      const trig = triggeredBy ?? (incremental ? "agent_idle" : "mount");
      if (!projectPath?.trim()) {
        return;
      }

      const userForced = trig === "user_refresh" || trig === "mount" || trig === "user_full";
      if (analyzeInFlightRef.current && !userForced) {
        return;
      }

      analyzeInFlightRef.current = true;
      setLifecycleMode(
        trig === "user_refresh" ? "manually_refreshing" : "generating",
      );
      setLoading(true);
      setError(null);
      try {
        const reg =
          registryRef.current.length > 0
            ? registryRef.current
            : await pirFetchNodeRegistry(port);
        registryRef.current = reg;
        setRegistry(reg);

        const analyzed = await pirAnalyze(
          port,
          chatId,
          projectPath,
          incremental,
          trig,
          chatContextRef.current ?? undefined,
        );

        applyGraphSnapshot(analyzed, trig, {
          clearSelection: !incremental,
          preservePositions: incremental,
        });
        pollStatusRef.current = "ready";

        if (
          enableLiveWatch &&
          analyzed.pir.provenance.project_path &&
          !watchStarted.current
        ) {
          watchStarted.current = true;
          void pirWatch(port, chatId, analyzed.pir.provenance.project_path);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        setLifecycleMode(resultRef.current ? "stable" : "idle");
      } finally {
        analyzeInFlightRef.current = false;
        setLoading(false);
      }
    },
    [port, chatId, projectPath, enableLiveWatch, applyGraphSnapshot],
  );

  const refreshDocument = useCallback(
    async (source = "poll") => {
      try {
        const { result: doc } = await pirDocument(port, chatId);
        applyGraphSnapshot(doc, source, {
          preservePositions: true,
        });
        pollStatusRef.current = "ready";
      } catch {
        /* best-effort poll */
      }
    },
    [port, chatId, applyGraphSnapshot],
  );

  const loadViewDocument = useCallback(
    async (view: GraphDiagramView): Promise<ViewDocument | null> => {
      if (view === "topology") {
        return viewDocumentsRef.current.topology ?? null;
      }
      const currentRevision = graphRevisionRef.current;
      const currentGraphVersion = graphVersionRef.current;
      const cached = viewDocumentsRef.current[view];
      if (
        cached &&
        isViewDocumentCurrent(currentRevision, currentGraphVersion, cached)
      ) {
        return cached;
      }
      const remote = await pirGraphViewDocument(
        port,
        chatId,
        toGraphViewApiName(view),
      );
      const parsed: ViewDocument = {
        revision: remote.revision,
        graphVersion: remote.graph_version,
        graph: remote.graph,
        mermaid: remote.mermaid,
        title: remote.title,
        generationError: remote.generation_error,
      };
      if (!isViewDocumentCurrent(currentRevision, currentGraphVersion, parsed)) {
        return null;
      }
      setViewDocuments((prev) => ({
        ...prev,
        [view]: parsed,
      }));
      return parsed;
    },
    [port, chatId],
  );

  useEffect(() => {
    if (!enabled || !chatId || !projectPath?.trim()) {
      mountedKeyRef.current = null;
      setLifecycleMode("idle");
      setViewDocuments({});
      return;
    }
    const key = `${chatId}:${projectPath.trim()}`;
    if (mountedKeyRef.current === key) return;
    mountedKeyRef.current = key;
    graphRevisionRef.current = null;
    graphVersionRef.current = null;
    lastAnalyzedTurnRef.current = null;
    setViewDocuments({});

    if (hydrateOnMount && skipMountAnalyze) {
      void refreshDocument("hydrate");
      return;
    }

    if (skipMountAnalyze) {
      return;
    }
    void runAnalyze(false, "mount");
  }, [enabled, chatId, projectPath, runAnalyze, skipMountAnalyze, hydrateOnMount, refreshDocument]);

  useEffect(() => {
    if (!reanalyzeWhenAgentIdle || !enabled || !projectPath?.trim() || !codegenReady) {
      return;
    }
    if (wasAgentStreaming.current && !isAgentStreaming) {
      if (!agentTurnId || lastAnalyzedTurnRef.current === agentTurnId) {
        wasAgentStreaming.current = isAgentStreaming;
        return;
      }
      if (agentIdleTimerRef.current) {
        window.clearTimeout(agentIdleTimerRef.current);
      }
      agentIdleTimerRef.current = window.setTimeout(() => {
        agentIdleTimerRef.current = null;
        lastAnalyzedTurnRef.current = agentTurnId;
        void runAnalyze(Boolean(resultRef.current), "agent_idle");
      }, 800);
    }
    wasAgentStreaming.current = isAgentStreaming;
    return () => {
      if (agentIdleTimerRef.current) {
        window.clearTimeout(agentIdleTimerRef.current);
        agentIdleTimerRef.current = null;
      }
    };
  }, [
    isAgentStreaming,
    agentTurnId,
    reanalyzeWhenAgentIdle,
    enabled,
    projectPath,
    chatId,
    runAnalyze,
    codegenReady,
  ]);

  /** Block mounts after main agent finishes — run first PIR analyze for this turn. */
  useEffect(() => {
    if (
      !skipMountAnalyze ||
      !reanalyzeWhenAgentIdle ||
      !enabled ||
      !projectPath?.trim() ||
      !codegenReady ||
      !agentTurnId ||
      isAgentStreaming
    ) {
      return;
    }
    if (lastAnalyzedTurnRef.current === agentTurnId) return;
    if (pirTurnAnalyzeDedupe.get(chatId) === agentTurnId) {
      lastAnalyzedTurnRef.current = agentTurnId;
      return;
    }

    lastAnalyzedTurnRef.current = agentTurnId;
    pirTurnAnalyzeDedupe.set(chatId, agentTurnId);
    void runAnalyze(Boolean(resultRef.current), "agent_turn");
  }, [
    skipMountAnalyze,
    reanalyzeWhenAgentIdle,
    enabled,
    projectPath,
    codegenReady,
    agentTurnId,
    isAgentStreaming,
    chatId,
    runAnalyze,
  ]);

  useEffect(() => {
    if (!chatId || pollMs <= 0) return;
    const id = window.setInterval(() => {
      void fetchPirStatus(port, chatId).then((s) => {
        setPirStatus(s);
        const prev = pollStatusRef.current;
        pollStatusRef.current = s.status;

        const revisionChanged =
          s.revision != null && s.revision !== graphRevisionRef.current;
        const graphVersionChanged =
          s.graph_version != null && s.graph_version !== graphVersionRef.current;
        const snapshotChanged = revisionChanged || graphVersionChanged;

        if (
          s.status === "ready" &&
          (prev === "analyzing" || prev === undefined) &&
          snapshotChanged
        ) {
          void refreshDocument("poll_revision_change");
        } else if (
          s.status === "ready" &&
          (prev === "analyzing" || prev === undefined) &&
          !revisionChanged
        ) {
          setLifecycleMode("stable");
        }

        if (s.status === "analyzing" && lifecycleMode !== "manually_refreshing") {
          setLifecycleMode("generating");
          if (!analyzeInFlightRef.current) {
            setLoading(true);
          }
        }
        if (s.status === "error" && s.error) {
          setError(s.error);
          setLoading(false);
          setLifecycleMode(resultRef.current ? "stable" : "idle");
        }
      });
    }, pollMs);
    return () => window.clearInterval(id);
  }, [port, chatId, pollMs, refreshDocument, lifecycleMode]);

  useEffect(() => {
    return () => {
      if (watchStarted.current) {
        void pirUnwatch(port, chatId);
        watchStarted.current = false;
      }
    };
  }, [port, chatId]);

  const selectedNode = useMemo(
    () => graph?.nodes.find((n) => n.id === selectedNodeId) ?? null,
    [graph, selectedNodeId],
  );

  const selectedPirNode = useMemo(
    () => pir?.nodes.find((n) => n.id === selectedNodeId) ?? null,
    [pir, selectedNodeId],
  );

  const selectedTypeDef = useMemo(
    () =>
      selectedNode
        ? (registry.find((r) => r.node_type === selectedNode.node_type) ?? null)
        : null,
    [registry, selectedNode],
  );

  const diffNodeIds = useMemo(() => {
    const d = result?.diff;
    if (!d) return new Set<string>();
    return new Set([...d.nodes_added, ...d.nodes_changed]);
  }, [result?.diff]);

  const updateGraph = useCallback((next: FirmwareGraph) => {
    setResult((prev) => (prev ? { ...prev, graph: next } : prev));
  }, []);

  const applyNodeEdits = useCallback(
    async (nodeId: string, updated: FirmwareNode) => {
      if (!pir || !result) return;
      const pirNode = pir.nodes.find((n) => n.id === nodeId);
      if (!pirNode) return;

      const oldProps = propsAsRecord(pirNode.properties);
      const newProps = propsAsRecord(updated.properties);
      const typeDef = registry.find((r) => r.node_type === updated.node_type);
      const keys = new Set<string>([
        ...pirNode.editable_fields,
        ...(typeDef?.properties.map((p) => p.key) ?? []),
      ]);

      const updates: Record<string, unknown> = {};
      for (const key of keys) {
        if (newProps[key] === undefined) continue;
        if (jsonStable(newProps[key]) !== jsonStable(oldProps[key])) {
          updates[key] = newProps[key];
        }
      }

      if (Object.keys(updates).length === 0) {
        updateGraph({
          ...result.graph,
          nodes: result.graph.nodes.map((n) => (n.id === nodeId ? updated : n)),
        });
        return;
      }

      setLoading(true);
      setError(null);
      setLifecycleMode("generating");
      try {
        const next = await pirApplyNodePatch(
          port,
          chatId,
          nodeId,
          updates,
          pir.revision,
        );
        applyGraphSnapshot(next, "patch", { preservePositions: true });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        setLifecycleMode("stable");
      } finally {
        setLoading(false);
      }
    },
    [pir, result, port, chatId, updateGraph, registry, applyGraphSnapshot],
  );

  const approveTopology = useCallback(
    async (comment?: string) => {
      await pirApprove(port, chatId, comment);
      await refreshDocument("approve");
    },
    [port, chatId, refreshDocument],
  );

  const runLayout = useCallback(() => {
    if (!graph) return;
    updateGraph(layoutGraph(graph, orientation));
  }, [graph, orientation, updateGraph]);

  const toggleOrientation = useCallback(() => {
    setOrientation((o) => (o === "horizontal" ? "vertical" : "horizontal"));
  }, []);

  useEffect(() => {
    if (!graph) return;
    updateGraph(layoutGraph(graph, orientation));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orientation]);

  return {
    chatId,
    graph,
    pir,
    result,
    registry,
    validation,
    pirStatus,
    lifecycleMode,
    selectedNodeId,
    selectedNode,
    selectedPirNode,
    selectedTypeDef,
    orientation,
    loading,
    error,
    diff: result?.diff,
    diffNodeIds,
    viewDocuments,
    loadViewDocument,
    setSelectedNodeId,
    updateGraph,
    applyNodeEdits,
    runAnalyze,
    runLayout,
    toggleOrientation,
    approveTopology,
  };
}
