import React, { memo, useCallback, useEffect, useMemo, useRef } from "react";
import type { SplitDiffRow } from "./diffUtils";
import styles from "./DiffContent.module.css";
import classNames from "classnames";

export const ROW_HEIGHT = 22;

type VirtualDiffScrollProps = {
  rows: SplitDiffRow[];
  side: "left" | "right" | "both";
  language: string;
  scrollTop: number;
  onScroll: (scrollTop: number) => void;
  scrollRef?: React.RefObject<HTMLDivElement>;
};

const DiffSideRow = memo(function DiffSideRow({
  row,
  side,
}: {
  row: SplitDiffRow;
  side: "left" | "right";
}) {
  const isLeft = side === "left";
  const text = isLeft ? row.leftText : row.rightText;
  const lineNumber = isLeft ? row.leftLine : row.rightLine;
  const sign =
    row.kind === "gap"
      ? " "
      : row.kind === "delete"
        ? "-"
        : row.kind === "insert"
          ? "+"
          : row.kind === "change"
            ? isLeft
              ? "-"
              : "+"
            : " ";

  const rowClass = useMemo(() => {
    if (row.kind === "gap") return styles.rowGap;
    if (row.kind === "delete") return styles.rowDelete;
    if (row.kind === "insert") return styles.rowInsert;
    if (row.kind === "change") {
      return isLeft ? styles.rowChangeLeft : styles.rowChangeRight;
    }
    return styles.rowContext;
  }, [isLeft, row.kind]);

  const displayText = text || (row.kind === "gap" ? "" : "\u00a0");

  return (
    <div className={classNames(styles.diffRow, rowClass)}>
      <div className={styles.lineNumber}>{lineNumber ?? ""}</div>
      <div className={styles.sign}>{sign}</div>
      <div className={styles.lineContent}>
        <span className={styles.lineText}>{displayText}</span>
      </div>
    </div>
  );
});

export const VirtualDiffScroll: React.FC<VirtualDiffScrollProps> = ({
  rows,
  side,
  language: _language,
  scrollTop,
  onScroll,
}) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  const syncingRef = useRef(false);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node || syncingRef.current) return;
    if (Math.abs(node.scrollTop - scrollTop) > 1) {
      syncingRef.current = true;
      node.scrollTop = scrollTop;
      syncingRef.current = false;
    }
  }, [scrollTop]);

  const handleScroll = useCallback(
    (event: React.UIEvent<HTMLDivElement>) => {
      if (syncingRef.current) return;
      onScroll(event.currentTarget.scrollTop);
    },
    [onScroll],
  );

  if (side === "both") {
    return (
      <div
        ref={scrollRef}
        className={styles.splitScrollArea}
        onScroll={handleScroll}
      >
        <div className={styles.splitBody}>
          <div className={styles.splitColumn}>
            <div className={styles.diffRows}>
              {rows.map((row) => (
                <DiffSideRow key={`left-${row.id}`} row={row} side="left" />
              ))}
            </div>
          </div>
          <div className={styles.splitColumn}>
            <div className={styles.diffRows}>
              {rows.map((row) => (
                <DiffSideRow key={`right-${row.id}`} row={row} side="right" />
              ))}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      className={styles.splitScrollArea}
      onScroll={handleScroll}
    >
      <div className={styles.diffRows}>
        {rows.map((row) => (
          <DiffSideRow key={`${side}-${row.id}`} row={row} side={side} />
        ))}
      </div>
    </div>
  );
};
