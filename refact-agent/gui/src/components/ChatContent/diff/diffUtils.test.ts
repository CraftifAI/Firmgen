import { describe, expect, test } from "vitest";
import {
  applyChunksToBefore,
  buildFullFileRowsFromLines,
  computeBeforeLinesFromAfter,
  countDiffLines,
  normalizeFileDiff,
  normalizeGroupedDiffs,
  resolveBeforeAfterLines,
  splitDiffLines,
  type DiffChunk,
} from "./diffUtils";

const SAMPLE_CHUNK: DiffChunk = {
  file_name: "/project/main.c",
  file_action: "edit",
  line1: 4,
  line2: 5,
  lines_remove: 'CONFIG_ESP_WIFI_PASSWORD=12345678\n',
  lines_add: 'CONFIG_ESP_WIFI_PASSWORD="12345678"\n',
};

describe("diffUtils", () => {
  test("splitDiffLines trims trailing newline", () => {
    expect(splitDiffLines("a\nb\n")).toEqual(["a", "b"]);
    expect(splitDiffLines("")).toEqual([]);
  });

  test("countDiffLines counts non-empty lines", () => {
    expect(countDiffLines(SAMPLE_CHUNK.lines_remove)).toBe(1);
    expect(countDiffLines(SAMPLE_CHUNK.lines_add)).toBe(1);
  });

  test("normalizeFileDiff builds split rows with change kind", () => {
    const file = normalizeFileDiff("/project/main.c", [SAMPLE_CHUNK]);
    expect(file.stats.additions).toBe(1);
    expect(file.stats.deletions).toBe(1);
    expect(file.splitRows).toHaveLength(1);
    expect(file.splitRows[0]?.kind).toBe("change");
    expect(file.splitRows[0]?.leftText).toContain("CONFIG_ESP_WIFI_PASSWORD=");
    expect(file.language).toBe("c");
  });

  test("normalizeGroupedDiffs groups by file path", () => {
    const grouped = normalizeGroupedDiffs({
      "/a.c": [SAMPLE_CHUNK],
      "/b.c": [{ ...SAMPLE_CHUNK, file_name: "/b.c" }],
    });
    expect(grouped).toHaveLength(2);
  });

  test("insert-only chunk produces insert rows", () => {
    const chunk: DiffChunk = {
      file_name: "/new.c",
      file_action: "add",
      line1: 1,
      line2: 2,
      lines_remove: "",
      lines_add: "int main() {\n  return 0;\n}\n",
    };
    const file = normalizeFileDiff("/new.c", [chunk]);
    expect(file.splitRows.every((row) => row.kind === "insert")).toBe(true);
    expect(file.stats.additions).toBe(3);
  });

  test("hydrated full rows include unchanged context", () => {
    const chunk: DiffChunk = {
      file_name: "/project/main.c",
      file_action: "edit",
      line1: 2,
      line2: 2,
      lines_remove: "old\n",
      lines_add: "new\n",
    };
    const afterLines = ["keep\n", "new\n", "tail\n"].map((l) => l.replace("\n", ""));
    const beforeLines = computeBeforeLinesFromAfter(afterLines, [chunk]);
    expect(beforeLines).toEqual(["keep", "old", "tail"]);
    const rows = buildFullFileRowsFromLines(beforeLines, afterLines);
    expect(rows).toHaveLength(3);
    expect(rows[0]?.kind).toBe("context");
    expect(rows[1]?.kind).toBe("change");
  });

  test("resolveBeforeAfterLines detects unapplied file content", () => {
    const chunk: DiffChunk = {
      file_name: "/project/main.c",
      file_action: "edit",
      line1: 2,
      line2: 2,
      lines_remove: "old\n",
      lines_add: "new\n",
    };
    const beforeLines = ["keep", "old", "tail"];
    const resolved = resolveBeforeAfterLines(beforeLines, [chunk]);
    expect(resolved.beforeLines).toEqual(beforeLines);
    expect(resolved.afterLines).toEqual(["keep", "new", "tail"]);
    expect(applyChunksToBefore(beforeLines, [chunk])).toEqual(resolved.afterLines);
  });

  test("resolveBeforeAfterLines detects applied file content", () => {
    const chunk: DiffChunk = {
      file_name: "/project/main.c",
      file_action: "edit",
      line1: 2,
      line2: 2,
      lines_remove: "old\n",
      lines_add: "new\n",
    };
    const afterLines = ["keep", "new", "tail"];
    const resolved = resolveBeforeAfterLines(afterLines, [chunk]);
    expect(resolved.afterLines).toEqual(afterLines);
    expect(resolved.beforeLines).toEqual(["keep", "old", "tail"]);
  });
});
