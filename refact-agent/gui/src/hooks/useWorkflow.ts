import { useState, useEffect, useCallback, useRef } from "react";
import { useConfig } from "./useConfig";
import type {
  WorkflowSnapshot,
  WorkflowEvent,
} from "../components/WorkflowPanel/types";

const DEFAULT_SNAPSHOT: WorkflowSnapshot = {
  has_active_workflow: false,
  workflow_id: null,
  workflow_name: null,
  workflow_type: null,
  device_context: null,
  progress: {
    total: 0,
    completed: 0,
    in_progress: 0,
    pending: 0,
    skipped: 0,
    failed: 0,
    percentage: 0,
  },
  tasks: [],
  can_pause: false,
  can_resume: false,
  is_paused: false,
};

interface UseWorkflowOptions {
  baseUrl?: string;
  pollingInterval?: number;
  enableSSE?: boolean;
  chatId?: string;
}

interface UseWorkflowReturn {
  snapshot: WorkflowSnapshot;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  cancel: () => Promise<void>;
  skipTask: (taskId: string) => Promise<void>;
  retryTask: (taskId: string) => Promise<void>;
  createWorkflow: (
    name: string,
    tasks?: Array<{ id?: string; description: string; tool?: string }>
  ) => Promise<void>;
}

export function useWorkflow(options: UseWorkflowOptions = {}): UseWorkflowReturn {
  const config = useConfig();
  const {
    baseUrl,
    pollingInterval = 5000,
    enableSSE = true,
    chatId,
  } = options;
  
  // Get base URL from config if not provided
  const apiBaseUrl = baseUrl || `http://127.0.0.1:${config.lspPort}`;

  const [snapshot, setSnapshot] = useState<WorkflowSnapshot>(DEFAULT_SNAPSHOT);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const eventSourceRef = useRef<EventSource | null>(null);
  const pollingRef = useRef<number | null>(null);

  // Fetch current workflow state
  const refresh = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);

      const url = chatId ? `${apiBaseUrl}/v1/workflow?chat_id=${encodeURIComponent(chatId)}` : `${apiBaseUrl}/v1/workflow`;
      const response = await fetch(url);
      const result = await response.json();

      if (result.success && result.data) {
        setSnapshot(result.data);
      } else if (result.error) {
        setError(result.error);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch workflow");
    } finally {
      setIsLoading(false);
    }
  }, [apiBaseUrl, chatId]);

  // Pause workflow
  const pause = useCallback(async () => {
    try {
      setIsLoading(true);
      const url = chatId ? `${apiBaseUrl}/v1/workflow/pause?chat_id=${encodeURIComponent(chatId)}` : `${apiBaseUrl}/v1/workflow/pause`;
      const response = await fetch(url, {
        method: "POST",
      });
      const result = await response.json();

      if (!result.success) {
        throw new Error(result.error || "Failed to pause workflow");
      }

      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to pause workflow");
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [apiBaseUrl, chatId, refresh]);

  // Resume workflow
  const resume = useCallback(async () => {
    try {
      setIsLoading(true);
      const url = chatId ? `${apiBaseUrl}/v1/workflow/resume?chat_id=${encodeURIComponent(chatId)}` : `${apiBaseUrl}/v1/workflow/resume`;
      const response = await fetch(url, {
        method: "POST",
      });
      const result = await response.json();

      if (!result.success) {
        throw new Error(result.error || "Failed to resume workflow");
      }

      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to resume workflow");
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [apiBaseUrl, chatId, refresh]);

  // Cancel workflow
  const cancel = useCallback(async () => {
    try {
      setIsLoading(true);
      const url = chatId ? `${apiBaseUrl}/v1/workflow?chat_id=${encodeURIComponent(chatId)}` : `${apiBaseUrl}/v1/workflow`;
      const response = await fetch(url, {
        method: "DELETE",
      });
      const result = await response.json();

      if (!result.success) {
        throw new Error(result.error || "Failed to cancel workflow");
      }

      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to cancel workflow");
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [apiBaseUrl, chatId, refresh]);

  // Skip a task
  const skipTask = useCallback(
    async (taskId: string) => {
      try {
        setIsLoading(true);
        const url = `${apiBaseUrl}/v1/workflow/tasks/${taskId}/action`;
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(chatId ? { chat_id: chatId, action: "skip", reason: "Skipped by user" } : { action: "skip", reason: "Skipped by user" }),
        });
        const result = await response.json();

        if (!result.success) {
          throw new Error(result.error || "Failed to skip task");
        }

        if (result.data) {
          setSnapshot(result.data);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to skip task");
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [apiBaseUrl, chatId]
  );

  // Retry a failed task
  const retryTask = useCallback(
    async (taskId: string) => {
      try {
        setIsLoading(true);
        const url = `${apiBaseUrl}/v1/workflow/tasks/${taskId}/action`;
        const response = await fetch(url, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(chatId ? { chat_id: chatId, action: "start" } : { action: "start" }),
        });
        const result = await response.json();

        if (!result.success) {
          throw new Error(result.error || "Failed to retry task");
        }

        if (result.data) {
          setSnapshot(result.data);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to retry task");
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [apiBaseUrl, chatId]
  );

  // Create a new workflow
  const createWorkflow = useCallback(
    async (
      name: string,
      tasks?: Array<{ id?: string; description: string; tool?: string }>
    ) => {
      try {
        setIsLoading(true);
        const response = await fetch(`${apiBaseUrl}/v1/workflow`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(chatId ? { chat_id: chatId, name, tasks } : { name, tasks }),
        });
        const result = await response.json();

        if (!result.success) {
          throw new Error(result.error || "Failed to create workflow");
        }

        if (result.data?.snapshot) {
          setSnapshot(result.data.snapshot);
        }
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to create workflow"
        );
        throw err;
      } finally {
        setIsLoading(false);
      }
    },
    [apiBaseUrl, chatId]
  );

  // Set up SSE for real-time updates
  useEffect(() => {
    if (!enableSSE) return;

    const setupSSE = () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }

      const eventsUrl = chatId ? `${apiBaseUrl}/v1/workflow/events?chat_id=${encodeURIComponent(chatId)}` : `${apiBaseUrl}/v1/workflow/events`;
      const eventSource = new EventSource(eventsUrl);

      eventSource.onmessage = (event) => {
        try {
          JSON.parse(event.data) as WorkflowEvent;
          refresh();
        } catch (err) {
          console.error("Failed to parse workflow event:", err);
        }
      };

      eventSource.onerror = () => {
        // SSE connection error - fall back to polling
        console.warn("Workflow SSE connection error, falling back to polling");
        eventSource.close();
        eventSourceRef.current = null;
      };

      eventSourceRef.current = eventSource;
    };

    setupSSE();

    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
        eventSourceRef.current = null;
      }
    };
  }, [apiBaseUrl, chatId, enableSSE, refresh]);

  // Set up polling as fallback
  useEffect(() => {
    // Initial fetch
    refresh();

    // Set up polling
    pollingRef.current = window.setInterval(() => {
      // Only poll if SSE is not connected
      if (!eventSourceRef.current || eventSourceRef.current.readyState !== EventSource.OPEN) {
        refresh();
      }
    }, pollingInterval);

    return () => {
      if (pollingRef.current) {
        clearInterval(pollingRef.current);
      }
    };
  }, [refresh, pollingInterval, apiBaseUrl, chatId]);

  return {
    snapshot,
    isLoading,
    error,
    refresh,
    pause,
    resume,
    cancel,
    skipTask,
    retryTask,
    createWorkflow,
  };
}

