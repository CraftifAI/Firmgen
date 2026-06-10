import React, { useMemo } from "react";
import { Box, Link, Text } from "@radix-ui/themes";
import type { NormalizedFileDiff } from "./diffUtils";
import { countVisibleChanges, type DiffViewMode } from "./diffUtils";
import { SplitDiffView, useChangeNavigation } from "./SplitDiffView";
import { useDiffFileHydration } from "./useDiffFileHydration";
import styles from "./DiffContent.module.css";
import { filename } from "../../../utils";
import { TruncateLeft } from "../../Text";
import { useEventsBusForIDE } from "../../../hooks";
import classNames from "classnames";

type DiffFileViewProps = {
  fileDiff: NormalizedFileDiff;
  viewMode: DiffViewMode;
  showUnchanged: boolean;
  enabled: boolean;
  onViewModeChange: (mode: DiffViewMode) => void;
  onShowUnchangedChange: (value: boolean) => void;
};

export const DiffFileView: React.FC<DiffFileViewProps> = ({
  fileDiff,
  viewMode,
  showUnchanged,
  enabled,
  onViewModeChange: _onViewModeChange,
  onShowUnchangedChange,
}) => {
  const { openFile } = useEventsBusForIDE();
  const { hydratedFileDiff, hydration } = useDiffFileHydration(
    fileDiff,
    enabled,
  );

  const changeCount = countVisibleChanges(
    hydratedFileDiff,
    viewMode,
    showUnchanged,
  );
  const { activeChangeIndex, goPrev, goNext } = useChangeNavigation(changeCount);

  const explanation = useMemo(() => {
    const details = hydratedFileDiff.stats;
    if (details.hunks === 0) return null;
    return `Updated ${filename(hydratedFileDiff.displayPath)} with ${details.additions} addition(s) and ${details.deletions} deletion(s) across ${details.hunks} hunk(s).`;
  }, [hydratedFileDiff]);

  const openPath = hydratedFileDiff.renameTarget ?? hydratedFileDiff.displayPath;

  const hydrationBanner = useMemo(() => {
    if (hydration.status === "loading") {
      return (
        <div className={styles.hydrationBanner}>
          Loading file content…
        </div>
      );
    }
    if (hydration.status === "error") {
      return (
        <div className={classNames(styles.hydrationBanner, styles.hydrationError)}>
          Could not load full file: {hydration.message}. Showing diff hunks only.
        </div>
      );
    }
    if (hydration.status === "ready" && hydration.truncated) {
      return (
        <div className={styles.hydrationBanner}>
          Large file truncated for performance. Open in editor for the full file.
        </div>
      );
    }
    return null;
  }, [hydration]);

  return (
    <Box className={styles.diffPanel}>
      {hydratedFileDiff.renameTarget && (
        <div className={styles.renameBanner}>
          Renamed {filename(hydratedFileDiff.displayPath)} →{" "}
          {filename(hydratedFileDiff.renameTarget)}
        </div>
      )}

      <div className={styles.fileHeader}>
        <TruncateLeft size="1" className={styles.filePathLink}>
          <Link
            href=""
            onClick={(event) => {
              event.preventDefault();
              openFile({ file_path: openPath, line: 1 });
            }}
          >
            {openPath}
          </Link>
        </TruncateLeft>

        <div className={styles.toolbar}>
          {/* <button
            type="button"
            className={classNames(styles.toolbarButton, {
              [styles.toolbarButtonActive]: viewMode === "split",
            })}
            onClick={() => onViewModeChange("split")}
          >
            Split
          </button>
          <button
            type="button"
            className={classNames(styles.toolbarButton, {
              [styles.toolbarButtonActive]: viewMode === "full",
            })}
            onClick={() => onViewModeChange("full")}
          >
            Full File
          </button> */}
          <button
            type="button"
            className={classNames(styles.toolbarButton, {
              [styles.toolbarButtonActive]: showUnchanged,
            })}
            onClick={() => onShowUnchangedChange(!showUnchanged)}
          >
            {showUnchanged ? "Hide unchanged" : "Show unchanged"}
          </button>
          <button
            type="button"
            className={styles.toolbarButton}
            onClick={goPrev}
            disabled={changeCount === 0}
          >
            ↑ Prev
          </button>
          <button
            type="button"
            className={styles.toolbarButton}
            onClick={goNext}
            disabled={changeCount === 0}
          >
            ↓ Next
          </button>
          <Text size="1" color="gray">
            {changeCount > 0
              ? `${activeChangeIndex + 1}/${changeCount}`
              : "0 changes"}
          </Text>
        </div>
      </div>

      {hydrationBanner}

      <SplitDiffView
        fileDiff={hydratedFileDiff}
        viewMode={viewMode}
        showUnchanged={showUnchanged}
        activeChangeIndex={activeChangeIndex}
        isLoading={hydration.status === "loading"}
      />

      {explanation && (
        <div className={styles.aiExplanation}>
          <div className={styles.aiExplanationTitle}>AI Explanation</div>
          <div className={styles.aiExplanationBody}>{explanation}</div>
        </div>
      )}
    </Box>
  );
};

type DiffFileTabsProps = {
  files: NormalizedFileDiff[];
  activeIndex: number;
  onSelect: (index: number) => void;
};

export const DiffFileTabs: React.FC<DiffFileTabsProps> = ({
  files,
  activeIndex,
  onSelect,
}) => {
  if (files.length <= 1) return null;

  return (
    <div className={styles.fileTabs}>
      {files.map((file, index) => (
        <button
          key={file.filePath}
          type="button"
          className={classNames(styles.fileTab, {
            [styles.fileTabActive]: index === activeIndex,
          })}
          onClick={() => onSelect(index)}
        >
          {filename(file.displayPath)}
          <span className={styles.diffSummaryStats}>
            <span className={styles.statAdd}>+{file.stats.additions}</span>
            <span className={styles.statDel}>-{file.stats.deletions}</span>
          </span>
        </button>
      ))}
    </div>
  );
};

export const DiffSummaryTitle: React.FC<{ files: NormalizedFileDiff[] }> = ({
  files,
}) => {
  const totals = files.reduce(
    (acc, file) => ({
      additions: acc.additions + file.stats.additions,
      deletions: acc.deletions + file.stats.deletions,
    }),
    { additions: 0, deletions: 0 },
  );

  const labels = files.map((file) => {
    if (file.renameTarget) {
      return `${filename(file.displayPath)} → ${filename(file.renameTarget)}`;
    }
    return filename(file.displayPath);
  });

  return (
    <div className={styles.summaryTitleInner}>
      <span className={styles.summaryFileCount}>
        {files.length} file{files.length === 1 ? "" : "s"}
      </span>
      <span className={styles.summaryFileNames}>{labels.join(", ")}</span>
      <span className={styles.diffSummaryStats}>
        <span className={styles.statAdd}>+{totals.additions}</span>
        <span className={styles.statDel}>-{totals.deletions}</span>
      </span>
    </div>
  );
};
