import React, { useCallback, useMemo, useRef, useState } from "react";
import { Container, Flex } from "@radix-ui/themes";
import { DiffMessage, type DiffChunk } from "../../services/refact";
import * as Collapsible from "@radix-ui/react-collapsible";
import { ChevronDownIcon } from "@radix-ui/react-icons";
import { FileDiff } from "lucide-react";
import groupBy from "lodash.groupby";
import { useHideScroll } from "../../hooks";
import {
  normalizeGroupedDiffs,
  type DiffViewMode,
} from "./diff/diffUtils";
import {
  DiffFileTabs,
  DiffFileView,
  DiffSummaryTitle,
} from "./diff/DiffFileView";
import diffStyles from "./diff/DiffContent.module.css";
import classNames from "classnames";

export type DiffChunkWithTypeAndApply = DiffChunk & {
  type: "apply" | "unapply" | "error" | "can not apply";
  apply: boolean;
};

export type DiffWithStatus = DiffChunk & {
  state?: 0 | 1 | 2;
  can_apply: boolean;
  applied: boolean;
  index: number;
};

/** @deprecated Use DiffSummaryTitle — kept for story/tests compatibility */
export const DiffTitle: React.FC<{
  diffs: Record<string, DiffChunk[]>;
}> = ({ diffs }) => {
  const files = useMemo(() => normalizeGroupedDiffs(diffs), [diffs]);
  return <DiffSummaryTitle files={files} />;
};

export const DiffContent: React.FC<{
  diffs: Record<string, DiffChunk[]>;
}> = ({ diffs }) => {
  const files = useMemo(() => normalizeGroupedDiffs(diffs), [diffs]);
  const [open, setOpen] = useState(true);
  const [activeFileIndex, setActiveFileIndex] = useState(0);
  const [viewMode, setViewMode] = useState<DiffViewMode>("split");
  const [showUnchanged, setShowUnchanged] = useState(true);
  const ref = useRef<HTMLButtonElement>(null);
  const handleScroll = useHideScroll(ref);

  const activeFile = files[activeFileIndex] ?? files[0];

  const handleHide = useCallback(() => {
    setOpen(false);
    handleScroll();
  }, [handleScroll]);

  const handleViewModeChange = useCallback((mode: DiffViewMode) => {
    setViewMode(mode);
    if (mode === "full") {
      setShowUnchanged(true);
    }
  }, []);

  if (files.length === 0) return null;

  const totalChanges = files.reduce(
    (acc, file) => acc + file.stats.additions + file.stats.deletions,
    0,
  );

  return (
    <Collapsible.Root open={open} onOpenChange={setOpen} className={diffStyles.diffDisclosure}>
      <Collapsible.Trigger asChild>
        <button
          type="button"
          ref={ref}
          className={classNames(diffStyles.diffDisclosureTrigger, {
            [diffStyles.diffDisclosureTriggerOpen]: open,
          })}
          aria-expanded={open}
        >
          <span className={diffStyles.diffDisclosureIconWrap}>
            <FileDiff size={16} strokeWidth={2} />
          </span>
          <span className={diffStyles.diffDisclosureBody}>
            <span className={diffStyles.diffDisclosureLabel}>File changes</span>
            <DiffSummaryTitle files={files} />
          </span>
          <span className={diffStyles.diffDisclosureMeta}>
            {totalChanges} line{totalChanges === 1 ? "" : "s"}
          </span>
          <ChevronDownIcon
            className={classNames(diffStyles.diffDisclosureChevron, {
              [diffStyles.diffDisclosureChevronOpen]: open,
            })}
          />
        </button>
      </Collapsible.Trigger>
      <Collapsible.Content className={diffStyles.diffDisclosureContent}>
        <Flex direction="column" gap="2" py="2" px="1">
          <DiffFileTabs
            files={files}
            activeIndex={activeFileIndex}
            onSelect={setActiveFileIndex}
          />
          {activeFile && (
            <DiffFileView
              fileDiff={activeFile}
              viewMode={viewMode}
              showUnchanged={showUnchanged}
              enabled={open}
              onViewModeChange={handleViewModeChange}
              onShowUnchangedChange={setShowUnchanged}
            />
          )}
          <button
            type="button"
            className={diffStyles.collapseFooterButton}
            onClick={handleHide}
          >
            Collapse diff
          </button>
        </Flex>
      </Collapsible.Content>
    </Collapsible.Root>
  );
};

/** @deprecated Inline diff rows replaced by split view — export retained for tests */
export const Diff: React.FC<{ diff: DiffChunk }> = () => null;

/** @deprecated Use DiffContent internals — export retained for tests */
export const DiffForm: React.FC<{
  diffs: Record<string, DiffChunk[]>;
}> = ({ diffs }) => {
  return <DiffContent diffs={diffs} />;
};

type GroupedDiffsProps = {
  diffs: DiffMessage[];
};

export const GroupedDiffs: React.FC<GroupedDiffsProps> = ({ diffs }) => {
  const chunks = diffs.reduce<DiffMessage["content"]>(
    (acc, diff) => [...acc, ...diff.content],
    [],
  );

  const groupedByFileName = groupBy(chunks, (chunk) => chunk.file_name);

  return (
    <Container>
      <Flex direction="column" gap="4" py="4">
        <DiffContent diffs={groupedByFileName} />
      </Flex>
    </Container>
  );
};
