import { useState, useEffect, useCallback } from "react";
import { useConfig } from "./useConfig";
import type { PipelineStage } from "./useWorkflowStatus";

export type WorkflowNode = "planning" | "generation" | "compiling" | "flashing" | "monitoring";

export interface ProgressEventDto {
  id: string;
  tool_name: string;
  operation: string;
  node: WorkflowNode;
  status: string;
  invoked_at_iso: string;
  completed_at_iso: string | null;
  summary: string | null;
  has_error: boolean;
  execution_duration_secs: number | null;
}

export interface ProgressSessionDto {
  chat_id: string;
  events: ProgressEventDto[];
  current_node: WorkflowNode;
  overall_percentage: number;
  activity_label: string;
  has_active_run: boolean;
  has_error: boolean;
  is_debugging: boolean;
  debug_iteration: number;
}

const NODE_ORDER: WorkflowNode[] = [
  "planning",
  "generation",
  "compiling",
  "flashing",
  "monitoring",
];

function getNodeOrdinal(node: WorkflowNode): number {
  const idx = NODE_ORDER.indexOf(node);
  return idx >= 0 ? idx : 0;
}

function toStorageKey(chatId: string): string {
  return `refact.progress.${chatId}`;
}

function loadPersistedProgress(chatId: string): ProgressSessionDto | null {
  try {
    const raw = window.localStorage.getItem(toStorageKey(chatId));
    if (!raw) return null;
    return JSON.parse(raw) as ProgressSessionDto;
  } catch {
    return null;
  }
}

function persistProgress(chatId: string, value: ProgressSessionDto): void {
  try {
    window.localStorage.setItem(toStorageKey(chatId), JSON.stringify(value));
  } catch {
    // Ignore storage quota/private mode failures.
  }
}

function keepMonotonicProgress(
  previous: ProgressSessionDto | null,
  next: ProgressSessionDto,
): ProgressSessionDto {
  if (!previous) return next;

  // New tool activity — trust the server (e.g. detect at flashing after monitoring).
  if (
    next.events.length > previous.events.length ||
    next.current_node !== previous.current_node ||
    next.has_active_run ||
    next.is_debugging
  ) {
    return next;
  }

  const prevOrdinal = getNodeOrdinal(previous.current_node);
  const nextOrdinal = getNodeOrdinal(next.current_node);
  if (nextOrdinal >= prevOrdinal) {
    return next;
  }

  return {
    ...next,
    current_node: previous.current_node,
    overall_percentage: Math.max(next.overall_percentage, previous.overall_percentage),
    events: next.events.length >= previous.events.length ? next.events : previous.events,
  };
}

function nodeToStage(node: WorkflowNode): PipelineStage {
  switch (node) {
    case "planning":
      return "PLANNING";
    case "generation":
      return "GENERATION";
    case "compiling":
      return "COMPILING";
    case "flashing":
      return "FLASH";
    case "monitoring":
      return "MONITORING";
    default:
      return "PLANNING";
  }
}

interface UseProgressOptions {
  baseUrl?: string;
  chatId?: string | null;
  pollingInterval?: number;
}

interface UseProgressReturn {
  progress: ProgressSessionDto | null;
  currentStage: PipelineStage;
  hasError: boolean;
  isStreaming: boolean;
  isDebugging: boolean;
  debugIteration: number;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  /** Call when the user submits a new message so the bar resets before new
   *  tool events arrive, rather than showing stale data from the previous run. */
  resetForNewRun: () => void;
}

export function useProgress(options: UseProgressOptions = {}): UseProgressReturn {
  const config = useConfig();
  const {
    baseUrl,
    chatId,
    pollingInterval = 500,
  } = options;

  const apiBaseUrl = baseUrl || `http://127.0.0.1:${config.lspPort}`;
  const [progress, setProgress] = useState<ProgressSessionDto | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [fetchError, setFetchError] = useState<string | null>(null);

  // When a new chat session starts, clear the previous progress immediately.
  // Otherwise the UI can briefly render the old session state (often showing
  // "planning" as completed because `progress` becomes null after the fetch).
  useEffect(() => {
    if (chatId) {
      setProgress(loadPersistedProgress(chatId));
    } else {
      setProgress(null);
    }
    setFetchError(null);
  }, [chatId]);

  const refresh = useCallback(async () => {
    if (!chatId) {
      setProgress(null);
      return;
    }
    try {
      setIsLoading(true);
      setFetchError(null);
      const url = `${apiBaseUrl}/v1/progress?chat_id=${encodeURIComponent(chatId)}`;
      const response = await fetch(url);
      const result = await response.json();
      if (result.success && result.data) {
        setProgress((previous) => {
          const merged = keepMonotonicProgress(
            previous,
            result.data as ProgressSessionDto,
          );
          persistProgress(chatId, merged);
          return merged;
        });
      } else {
        // Keep the last known state to prevent visual regressions.
      }
    } catch (err) {
      setFetchError(err instanceof Error ? err.message : "Failed to fetch progress");
      // Keep previously known progress on transient network/server errors.
    } finally {
      setIsLoading(false);
    }
  }, [apiBaseUrl, chatId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!chatId) return;
    const timer = window.setInterval(refresh, pollingInterval);
    return () => window.clearInterval(timer);
  }, [chatId, pollingInterval, refresh]);

  // Clear the persisted localStorage entry and wipe progress state so the
  // bar shows empty circles when the user starts a new agent turn.
  // We do NOT immediately blank the bar mid-render — we let the next poll
  // return fresh data so the transition is smooth rather than jarring.
  const resetForNewRun = useCallback(() => {
    if (chatId) {
      try {
        window.localStorage.removeItem(toStorageKey(chatId));
      } catch {
        // ignore storage errors
      }
    }
    setProgress(null);
  }, [chatId]);

  const currentStage = progress ? nodeToStage(progress.current_node) : "PLANNING";
  const hasError = progress?.has_error ?? false;
  const isStreaming = progress?.has_active_run ?? false;
  const isDebugging = progress?.is_debugging ?? false;
  const debugIteration = progress?.debug_iteration ?? 0;

  return {
    progress,
    currentStage,
    hasError,
    isStreaming,
    isDebugging,
    debugIteration,
    isLoading,
    error: fetchError,
    refresh,
    resetForNewRun,
  };
}
