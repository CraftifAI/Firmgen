import type { DiffChunk } from "../../../services/refact";
import { filename } from "../../../utils";

export type { DiffChunk };

export type DiffRowKind = "context" | "change" | "delete" | "insert" | "gap";

export type SplitDiffRow = {
  id: string;
  kind: DiffRowKind;
  leftLine?: number;
  rightLine?: number;
  leftText: string;
  rightText: string;
  hunkIndex: number;
};

export type FileDiffStats = {
  additions: number;
  deletions: number;
  hunks: number;
};

export type DiffViewMode = "split" | "full";

export type NormalizedFileDiff = {
  filePath: string;
  displayPath: string;
  language: string;
  fileAction: string;
  renameTarget?: string;
  chunks: DiffChunk[];
  splitRows: SplitDiffRow[];
  fullRows: SplitDiffRow[];
  hydratedFullRows?: SplitDiffRow[];
  hydratedSplitRows?: SplitDiffRow[];
  stats: FileDiffStats;
  changeRowIndices: number[];
};

export type FileHydrationState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; beforeText: string; afterText: string; truncated: boolean }
  | { status: "error"; message: string };

function normalizeChunks(chunks: DiffChunk[]): DiffChunk[] {
  return chunks.map((chunk) => ({
    ...chunk,
    line1: chunk.line1 > 0 ? chunk.line1 : 1,
    line2:
      chunk.line2 > 0
        ? chunk.line2
        : chunk.line1 > 0
          ? chunk.line1
          : 1,
  }));
}

function getEditChunks(chunks: DiffChunk[]): DiffChunk[] {
  return normalizeChunks(chunks)
    .filter((chunk) => chunk.file_action !== "rename")
    .slice();
}

export function applyChunksToBefore(
  beforeLines: string[],
  chunks: DiffChunk[],
): string[] {
  const lines = [...beforeLines];
  const editChunks = getEditChunks(chunks).sort((a, b) => b.line1 - a.line1);

  for (const chunk of editChunks) {
    const removed = splitDiffLines(chunk.lines_remove);
    const added = splitDiffLines(chunk.lines_add);
    if (removed.length === 0 && added.length === 0) continue;
    const startIdx = Math.max(0, chunk.line1 - 1);
    lines.splice(startIdx, removed.length, ...added);
  }

  return lines;
}

export function computeBeforeLinesFromAfter(
  afterLines: string[],
  chunks: DiffChunk[],
): string[] {
  const lines = [...afterLines];
  const editChunks = getEditChunks(chunks).sort((a, b) => b.line1 - a.line1);

  for (const chunk of editChunks) {
    const removed = splitDiffLines(chunk.lines_remove);
    const added = splitDiffLines(chunk.lines_add);
    if (removed.length === 0 && added.length === 0) continue;
    const startIdx = Math.max(0, chunk.line1 - 1);
    if (removed.length === 0) {
      lines.splice(startIdx, added.length);
      continue;
    }
    if (added.length === 0) {
      lines.splice(startIdx, 0, ...removed);
      continue;
    }
    lines.splice(startIdx, added.length, ...removed);
  }

  return lines;
}

function linesEqual(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((line, index) => line === b[index]);
}

export function resolveBeforeAfterLines(
  fileLines: string[],
  chunks: DiffChunk[],
): { beforeLines: string[]; afterLines: string[] } {
  const editChunks = getEditChunks(chunks);
  if (editChunks.length === 0) {
    return { beforeLines: fileLines, afterLines: fileLines };
  }

  const beforeFromReverse = computeBeforeLinesFromAfter(fileLines, editChunks);
  const afterFromForward = applyChunksToBefore(fileLines, editChunks);

  const reverseRoundTrip = applyChunksToBefore(beforeFromReverse, editChunks);
  const forwardRoundTrip = computeBeforeLinesFromAfter(
    afterFromForward,
    editChunks,
  );

  const reverseMatches = linesEqual(reverseRoundTrip, fileLines);
  const forwardMatches = linesEqual(forwardRoundTrip, fileLines);

  if (forwardMatches && !reverseMatches) {
    return { beforeLines: fileLines, afterLines: afterFromForward };
  }

  if (reverseMatches && !forwardMatches) {
    return { beforeLines: beforeFromReverse, afterLines: fileLines };
  }

  if (forwardMatches && reverseMatches) {
    const allInsertOnly = editChunks.every(
      (chunk) =>
        splitDiffLines(chunk.lines_remove).length === 0 &&
        splitDiffLines(chunk.lines_add).length > 0,
    );
    if (allInsertOnly && beforeFromReverse.length < fileLines.length) {
      return { beforeLines: beforeFromReverse, afterLines: fileLines };
    }
    return { beforeLines: beforeFromReverse, afterLines: fileLines };
  }

  const reverseDelta = Math.abs(reverseRoundTrip.length - fileLines.length);
  const forwardDelta = Math.abs(forwardRoundTrip.length - fileLines.length);
  if (forwardDelta < reverseDelta) {
    return { beforeLines: fileLines, afterLines: afterFromForward };
  }
  return { beforeLines: beforeFromReverse, afterLines: fileLines };
}

export function buildFullFileRowsFromLines(
  beforeLines: string[],
  afterLines: string[],
): SplitDiffRow[] {
  const maxLen = Math.max(beforeLines.length, afterLines.length, 0);
  if (maxLen === 0) return [];

  const rows: SplitDiffRow[] = [];
  for (let index = 0; index < maxLen; index++) {
    const leftText = index < beforeLines.length ? beforeLines[index] : "";
    const rightText = index < afterLines.length ? afterLines[index] : "";
    const lineNum = index + 1;
    const hasLeft = index < beforeLines.length;
    const hasRight = index < afterLines.length;

    let kind: DiffRowKind = "context";
    if (hasLeft && hasRight) {
      kind = leftText === rightText ? "context" : "change";
    } else if (hasLeft) {
      kind = "delete";
    } else if (hasRight) {
      kind = "insert";
    }

    rows.push({
      id: `hydrated-full-${lineNum}`,
      kind,
      leftLine: hasLeft ? lineNum : undefined,
      rightLine: hasRight ? lineNum : undefined,
      leftText,
      rightText,
      hunkIndex: 0,
    });
  }

  return rows;
}

export function buildChangedOnlyRows(fullRows: SplitDiffRow[]): SplitDiffRow[] {
  return fullRows.filter(
    (row) => row.kind !== "context" && row.kind !== "gap",
  );
}

export function applyHydrationToFileDiff(
  fileDiff: NormalizedFileDiff,
  hydration: Extract<FileHydrationState, { status: "ready" }>,
): NormalizedFileDiff {
  const fileLines = splitDiffLines(hydration.afterText);
  const { beforeLines, afterLines } = resolveBeforeAfterLines(
    fileLines,
    fileDiff.chunks,
  );
  const hydratedFullRows = buildFullFileRowsFromLines(beforeLines, afterLines);
  const hydratedSplitRows = buildChangedOnlyRows(hydratedFullRows);

  return {
    ...fileDiff,
    hydratedFullRows,
    hydratedSplitRows,
    changeRowIndices: collectChangeIndices(
      hydratedSplitRows.length > 0 ? hydratedSplitRows : hydratedFullRows,
    ),
  };
}

const EXTENSION_LANGUAGE: Record<string, string> = {
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  hpp: "cpp",
  rs: "rust",
  py: "python",
  js: "javascript",
  jsx: "jsx",
  ts: "typescript",
  tsx: "tsx",
  json: "json",
  md: "markdown",
  sh: "bash",
  bash: "bash",
  yml: "yaml",
  yaml: "yaml",
  xml: "xml",
  html: "html",
  css: "css",
  cmake: "cmake",
  txt: "plaintext",
  defaults: "plaintext",
};

export function normalizeDisplayPath(filePath: string): string {
  return filePath.replace(/^\\\\\?\\/, "");
}

export function detectLanguageFromPath(filePath: string): string {
  const base = filename(normalizeDisplayPath(filePath));
  if (base.toLowerCase().startsWith("dockerfile")) return "dockerfile";
  if (base.toLowerCase().startsWith("cmakelists")) return "cmake";
  if (base.toLowerCase().startsWith("sdkconfig")) return "plaintext";
  const parts = base.split(".");
  if (parts.length < 2) return "plaintext";
  const ext = parts[parts.length - 1].toLowerCase();
  return EXTENSION_LANGUAGE[ext] ?? ext;
}

export function splitDiffLines(text: string): string[] {
  if (!text) return [];
  const lines = text.split("\n");
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }
  return lines;
}

export function countDiffLines(text: string): number {
  return splitDiffLines(text).length;
}

function alignHunkLines(
  removed: string[],
  added: string[],
  startLine: number,
  hunkIndex: number,
): SplitDiffRow[] {
  const maxLen = Math.max(removed.length, added.length, 1);
  const rows: SplitDiffRow[] = [];

  for (let i = 0; i < maxLen; i++) {
    const leftText = i < removed.length ? removed[i] : "";
    const rightText = i < added.length ? added[i] : "";
    const hasLeft = i < removed.length;
    const hasRight = i < added.length;

    let kind: DiffRowKind = "context";
    if (hasLeft && hasRight) {
      kind = leftText === rightText ? "context" : "change";
    } else if (hasLeft) {
      kind = "delete";
    } else if (hasRight) {
      kind = "insert";
    }

    if (!hasLeft && !hasRight) continue;

    rows.push({
      id: `hunk-${hunkIndex}-row-${i}`,
      kind,
      leftLine: hasLeft ? startLine + i : undefined,
      rightLine: hasRight ? startLine + i : undefined,
      leftText,
      rightText,
      hunkIndex,
    });
  }

  return rows;
}

function buildGapRow(
  gapLines: number,
  hunkIndex: number,
  rowIndex: number,
): SplitDiffRow {
  const label =
    gapLines === 1
      ? "⋯ 1 unchanged line ⋯"
      : `⋯ ${gapLines} unchanged lines ⋯`;
  return {
    id: `gap-${hunkIndex}-${rowIndex}`,
    kind: "gap",
    leftText: label,
    rightText: label,
    hunkIndex,
  };
}

function buildSplitRows(chunks: DiffChunk[]): SplitDiffRow[] {
  const editChunks = normalizeChunks(chunks)
    .filter((chunk) => chunk.file_action !== "rename")
    .slice()
    .sort((a, b) => a.line1 - b.line1);

  const rows: SplitDiffRow[] = [];
  let previousEndLine = 0;

  editChunks.forEach((chunk, hunkIndex) => {
    const removed = splitDiffLines(chunk.lines_remove);
    const added = splitDiffLines(chunk.lines_add);

    if (removed.length === 0 && added.length === 0) return;

    const gap = chunk.line1 - previousEndLine - 1;
    if (gap > 0 && previousEndLine > 0) {
      rows.push(buildGapRow(gap, hunkIndex, rows.length));
    }

    rows.push(...alignHunkLines(removed, added, chunk.line1, hunkIndex));
    previousEndLine = Math.max(
      chunk.line2,
      chunk.line1 + Math.max(removed.length, added.length) - 1,
    );
  });

  return rows;
}

function reconstructSideLines(
  chunks: DiffChunk[],
  side: "before" | "after",
): Map<number, string> {
  const lineMap = new Map<number, string>();
  const editChunks = normalizeChunks(chunks)
    .filter((chunk) => chunk.file_action !== "rename")
    .slice()
    .sort((a, b) => a.line1 - b.line1);

  for (const chunk of editChunks) {
    const lines =
      side === "before"
        ? splitDiffLines(chunk.lines_remove)
        : splitDiffLines(chunk.lines_add);
    lines.forEach((line, index) => {
      lineMap.set(chunk.line1 + index, line);
    });
  }

  return lineMap;
}

function buildFullFileRows(chunks: DiffChunk[]): SplitDiffRow[] {
  const beforeMap = reconstructSideLines(chunks, "before");
  const afterMap = reconstructSideLines(chunks, "after");

  if (beforeMap.size === 0 && afterMap.size === 0) {
    return [];
  }

  const allLineNumbers = new Set<number>([
    ...beforeMap.keys(),
    ...afterMap.keys(),
  ]);
  const sortedLines = [...allLineNumbers].sort((a, b) => a - b);

  const rows: SplitDiffRow[] = [];
  let previousLine = 0;

  sortedLines.forEach((lineNumber, index) => {
    const gap = lineNumber - previousLine - 1;
    if (gap > 0 && previousLine > 0) {
      rows.push(buildGapRow(gap, index, rows.length));
    }

    const leftText = beforeMap.get(lineNumber) ?? "";
    const rightText = afterMap.get(lineNumber) ?? "";
    const hasLeft = beforeMap.has(lineNumber);
    const hasRight = afterMap.has(lineNumber);

    let kind: DiffRowKind = "context";
    if (hasLeft && hasRight) {
      kind = leftText === rightText ? "context" : "change";
    } else if (hasLeft) {
      kind = "delete";
    } else if (hasRight) {
      kind = "insert";
    }

    rows.push({
      id: `full-${lineNumber}`,
      kind,
      leftLine: hasLeft ? lineNumber : undefined,
      rightLine: hasRight ? lineNumber : undefined,
      leftText,
      rightText,
      hunkIndex: index,
    });

    previousLine = lineNumber;
  });

  return rows;
}

function computeStats(chunks: DiffChunk[]): FileDiffStats {
  let additions = 0;
  let deletions = 0;
  let hunks = 0;

  for (const chunk of chunks) {
    if (chunk.file_action === "rename") continue;
    const removed = countDiffLines(chunk.lines_remove);
    const added = countDiffLines(chunk.lines_add);
    if (removed > 0 || added > 0) {
      hunks += 1;
    }
    additions += added;
    deletions += removed;
  }

  return { additions, deletions, hunks };
}

function collectChangeIndices(rows: SplitDiffRow[]): number[] {
  return rows.reduce<number[]>((acc, row, index) => {
    if (row.kind === "gap" || row.kind === "context") return acc;
    return [...acc, index];
  }, []);
}

export function normalizeFileDiff(
  filePath: string,
  chunks: DiffChunk[],
): NormalizedFileDiff {
  const normalizedChunks = normalizeChunks(chunks);
  const renameAction = normalizedChunks.find(
    (chunk) => chunk.file_action === "rename" && chunk.file_name_rename,
  );

  const splitRows = buildSplitRows(normalizedChunks);
  const fullRows = buildFullFileRows(normalizedChunks);
  const rowsForChanges = splitRows.length > 0 ? splitRows : fullRows;

  return {
    filePath,
    displayPath: normalizeDisplayPath(filePath),
    language: detectLanguageFromPath(filePath),
    fileAction:
      renameAction?.file_action ?? normalizedChunks[0]?.file_action ?? "edit",
    renameTarget: renameAction?.file_name_rename ?? undefined,
    chunks: normalizedChunks,
    splitRows,
    fullRows,
    stats: computeStats(normalizedChunks),
    changeRowIndices: collectChangeIndices(rowsForChanges),
  };
}

export function normalizeGroupedDiffs(
  grouped: Record<string, DiffChunk[]>,
): NormalizedFileDiff[] {
  return Object.entries(grouped).map(([filePath, chunks]) =>
    normalizeFileDiff(filePath, chunks),
  );
}

export function computeTotalChangedLines(stats: FileDiffStats): number {
  return stats.additions + stats.deletions;
}

export function shouldAutoExpandDiff(files: NormalizedFileDiff[]): boolean {
  const totalRows = files.reduce(
    (acc, file) => acc + file.splitRows.length,
    0,
  );
  return totalRows > 0 && totalRows <= 24;
}

export function filterRowsByUnchanged(
  rows: SplitDiffRow[],
  showUnchanged: boolean,
): SplitDiffRow[] {
  if (showUnchanged) return rows;
  return rows.filter((row) => row.kind !== "gap" && row.kind !== "context");
}

export function selectDiffRows(
  fileDiff: NormalizedFileDiff,
  viewMode: DiffViewMode,
  showUnchanged: boolean,
): SplitDiffRow[] {
  let base: SplitDiffRow[];
  if (viewMode === "full") {
    base =
      fileDiff.hydratedFullRows && fileDiff.hydratedFullRows.length > 0
        ? fileDiff.hydratedFullRows
        : fileDiff.splitRows.length > 0
          ? fileDiff.splitRows
          : fileDiff.fullRows;
  } else {
    base =
      fileDiff.hydratedFullRows && fileDiff.hydratedFullRows.length > 0
        ? fileDiff.hydratedFullRows
        : fileDiff.splitRows.length > 0
          ? fileDiff.splitRows
          : fileDiff.hydratedSplitRows &&
              fileDiff.hydratedSplitRows.length > 0
            ? fileDiff.hydratedSplitRows
            : fileDiff.fullRows;
  }

  if (base.length === 0) {
    const fallback =
      viewMode === "full" ? fileDiff.splitRows : fileDiff.fullRows;
    return filterRowsByUnchanged(fallback, showUnchanged);
  }
  return filterRowsByUnchanged(base, showUnchanged);
}

export function countVisibleChanges(
  fileDiff: NormalizedFileDiff,
  viewMode: DiffViewMode,
  showUnchanged: boolean,
): number {
  return selectDiffRows(fileDiff, viewMode, showUnchanged).filter(
    (row) => row.kind !== "gap" && row.kind !== "context",
  ).length;
}
