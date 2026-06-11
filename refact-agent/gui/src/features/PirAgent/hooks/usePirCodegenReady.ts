import { useEffect, useRef, useState } from "react";

import { useConfig } from "../../../hooks";
import {
  fetchProjectTree,
  invalidateProjectTreeCache,
  type ProjectTreeNode,
} from "../../../services/refact/projectTree";

function treeHasCodegenArtifact(nodes: ProjectTreeNode[]): boolean {
  for (const node of nodes) {
    const name = node.name.toLowerCase();
    const normPath = node.path.replace(/\\/g, "/").toLowerCase();
    if (node.type === "file") {
      // app_config.h is the strongest signal — PIR_maker reads it as the topology manifest
      if (name === "app_config.h" || normPath.endsWith("/main/app_config.h")) {
        return true;
      }
      // Any C/C++ source file directly inside main/ counts — matches backend
      // main_dir_has_sources which accepts station_example_main.c, gatt_server_main.c, etc.
      if (
        normPath.includes("/main/") &&
        (name.endsWith(".c") || name.endsWith(".cpp") || name.endsWith(".cc"))
      ) {
        return true;
      }
    }
    if (node.type === "dir" && node.children?.length) {
      if (treeHasCodegenArtifact(node.children)) return true;
    }
  }
  return false;
}

export type UsePirCodegenReadyOptions = {
  chatId: string | null;
  projectPath: string | null;
  /** Latest completed assistant turn — re-check when this changes. */
  agentTurnId: string | null;
  /** True while main agent is still streaming/waiting. */
  agentIsWorking: boolean;
};

const MAX_RETRIES = 10;
const RETRY_MS = 1500;

/**
 * True once main agent has produced project code (app_config.h or main/*.c).
 * Latches to true for the chat session so build/flash does not hide the gate.
 */
export function usePirCodegenReady({
  chatId,
  projectPath,
  agentTurnId,
  agentIsWorking,
}: UsePirCodegenReadyOptions): {
  ready: boolean;
  checking: boolean;
} {
  const config = useConfig();
  const port = config.lspPort;
  const [ready, setReady] = useState(false);
  const [checking, setChecking] = useState(false);
  const latchedReadyRef = useRef(false);
  const lastCheckedTurnRef = useRef<string | null>(null);
  const retryAttemptRef = useRef(0);
  const retryTimerRef = useRef<number | null>(null);

  useEffect(() => {
    latchedReadyRef.current = false;
    lastCheckedTurnRef.current = null;
    retryAttemptRef.current = 0;
    setReady(false);
    setChecking(false);
  }, [chatId]);

  useEffect(() => {
    if (retryTimerRef.current) {
      window.clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }

    const path = projectPath?.trim() ?? "";
    if (!path || !agentTurnId) {
      return;
    }

    if (latchedReadyRef.current) {
      setReady(true);
      return;
    }

    if (agentIsWorking) {
      return;
    }

    if (lastCheckedTurnRef.current === agentTurnId && ready) {
      return;
    }

    let cancelled = false;

    const runCheck = (attempt: number) => {
      if (cancelled) return;
      setChecking(true);
      if (attempt === 0) {
        invalidateProjectTreeCache(path, port, 4);
      }

      void fetchProjectTree(path, port, 4, { force: attempt === 0 })
        .then(({ tree }) => {
          if (cancelled) return;
          const found = treeHasCodegenArtifact(tree);
          if (found) {
            latchedReadyRef.current = true;
            setReady(true);
            lastCheckedTurnRef.current = agentTurnId;
            retryAttemptRef.current = 0;
          } else {
            setReady(false);
          }

          if (found) return;

          if (attempt + 1 < MAX_RETRIES) {
            retryAttemptRef.current = attempt + 1;
            retryTimerRef.current = window.setTimeout(() => {
              runCheck(attempt + 1);
            }, RETRY_MS);
          } else {
            retryAttemptRef.current = 0;
          }
        })
        .catch(() => {
          if (!cancelled && !latchedReadyRef.current) setReady(false);
        })
        .finally(() => {
          if (!cancelled) setChecking(false);
        });
    };

    runCheck(retryAttemptRef.current);

    return () => {
      cancelled = true;
      if (retryTimerRef.current) {
        window.clearTimeout(retryTimerRef.current);
        retryTimerRef.current = null;
      }
    };
  }, [projectPath, agentTurnId, agentIsWorking, port, ready, chatId]);

  return { ready, checking };
}
