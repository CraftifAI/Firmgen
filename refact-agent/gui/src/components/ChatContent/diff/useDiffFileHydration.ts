import { useEffect, useMemo, useState } from "react";
import { useAppSelector } from "../../../hooks";
import { fetchWorkspaceFileContent } from "../../../services/refact/workspaceFile";
import {
  applyHydrationToFileDiff,
  resolveBeforeAfterLines,
  type FileHydrationState,
  type NormalizedFileDiff,
  splitDiffLines,
} from "./diffUtils";

export function useDiffFileHydration(
  fileDiff: NormalizedFileDiff,
  enabled: boolean,
): {
  hydratedFileDiff: NormalizedFileDiff;
  hydration: FileHydrationState;
} {
  const port = useAppSelector((state) => state.config.lspPort);
  const [hydration, setHydration] = useState<FileHydrationState>({
    status: "idle",
  });

  const filePath = fileDiff.filePath;
  const chunksKey = useMemo(
    () => JSON.stringify(fileDiff.chunks),
    [fileDiff.chunks],
  );

  useEffect(() => {
    if (!enabled || !filePath.trim()) {
      setHydration({ status: "idle" });
      return;
    }

    let cancelled = false;
    setHydration({ status: "loading" });

    fetchWorkspaceFileContent(filePath, port)
      .then((response) => {
        if (cancelled) return;
        const { beforeLines, afterLines } = resolveBeforeAfterLines(
          splitDiffLines(response.content),
          fileDiff.chunks,
        );
        setHydration({
          status: "ready",
          afterText:
            afterLines.join("\n") + (afterLines.length > 0 ? "\n" : ""),
          beforeText:
            beforeLines.join("\n") + (beforeLines.length > 0 ? "\n" : ""),
          truncated: response.truncated,
        });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        const message =
          error instanceof Error ? error.message : "Failed to load file content";
        setHydration({ status: "error", message });
      });

    return () => {
      cancelled = true;
    };
  }, [enabled, filePath, port, chunksKey, fileDiff.chunks]);

  const hydratedFileDiff = useMemo(() => {
    if (hydration.status !== "ready") return fileDiff;
    return applyHydrationToFileDiff(fileDiff, hydration);
  }, [fileDiff, hydration]);

  return { hydratedFileDiff, hydration };
}
