import React, { useMemo, useState } from "react";
import type { NormalizedFileDiff } from "./diffUtils";
import { selectDiffRows, type DiffViewMode } from "./diffUtils";
import { VirtualDiffScroll, ROW_HEIGHT } from "./VirtualDiffScroll";
import styles from "./DiffContent.module.css";

type SplitDiffViewProps = {
  fileDiff: NormalizedFileDiff;
  viewMode: DiffViewMode;
  showUnchanged: boolean;
  activeChangeIndex: number;
  isLoading?: boolean;
};

export const SplitDiffView: React.FC<SplitDiffViewProps> = ({
  fileDiff,
  viewMode,
  showUnchanged,
  activeChangeIndex,
  isLoading = false,
}) => {
  const [scrollTop, setScrollTop] = useState(0);

  const rows = useMemo(
    () => selectDiffRows(fileDiff, viewMode, showUnchanged),
    [fileDiff, showUnchanged, viewMode],
  );

  const changeIndices = useMemo(
    () =>
      rows.reduce<number[]>((acc, row, index) => {
        if (row.kind === "gap" || row.kind === "context") return acc;
        return [...acc, index];
      }, []),
    [rows],
  );

  React.useEffect(() => {
    if (changeIndices.length === 0) return;
    const bounded = Math.min(activeChangeIndex, changeIndices.length - 1);
    const rowIndex = changeIndices[bounded] ?? 0;
    setScrollTop(Math.max(0, rowIndex * ROW_HEIGHT - ROW_HEIGHT * 2));
  }, [activeChangeIndex, changeIndices]);

  const minimapMarkers = useMemo(() => {
    if (rows.length === 0) return [];
    return changeIndices.map((rowIndex) => {
      const row = rows[rowIndex];
      const top = (rowIndex / rows.length) * 100;
      const height = Math.max(100 / rows.length, 1.5);
      const markerClass =
        row.kind === "delete"
          ? styles.minimapDelete
          : row.kind === "insert"
            ? styles.minimapInsert
            : styles.minimapChange;
      return { key: row.id, top, height, markerClass };
    });
  }, [changeIndices, rows]);

  if (isLoading && rows.length === 0) {
    return (
      <div className={styles.splitContainer}>
        <div className={styles.loadingState}>Loading file content…</div>
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className={styles.splitContainer}>
        <div className={styles.renameBanner}>No diff lines to display.</div>
      </div>
    );
  }

  return (
    <div className={styles.splitContainer}>
      <div className={styles.splitHeaders}>
        <div
          className={`${styles.splitPaneHeader} ${styles.splitPaneHeaderLeft}`}
        >
          Your Code
        </div>
        <div
          className={`${styles.splitPaneHeader} ${styles.splitPaneHeaderRight}`}
        >
          AI Generated
        </div>
      </div>

      <VirtualDiffScroll
        rows={rows}
        side="both"
        language={fileDiff.language}
        scrollTop={scrollTop}
        onScroll={setScrollTop}
      />

      {rows.length >= 12 && (
        <div className={styles.minimap} aria-hidden>
          {minimapMarkers.map((marker) => (
            <div
              key={marker.key}
              className={`${styles.minimapMarker} ${marker.markerClass}`}
              style={{
                top: `${marker.top}%`,
                height: `${marker.height}%`,
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export function useChangeNavigation(changeCount: number) {
  const [activeChangeIndex, setActiveChangeIndex] = React.useState(0);

  const goPrev = React.useCallback(() => {
    setActiveChangeIndex((current) =>
      changeCount === 0 ? 0 : (current - 1 + changeCount) % changeCount,
    );
  }, [changeCount]);

  const goNext = React.useCallback(() => {
    setActiveChangeIndex((current) =>
      changeCount === 0 ? 0 : (current + 1) % changeCount,
    );
  }, [changeCount]);

  return { activeChangeIndex, goPrev, goNext };
}
