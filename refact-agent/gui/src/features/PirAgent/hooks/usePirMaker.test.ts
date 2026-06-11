import { describe, expect, it } from "vitest";

import { isViewDocumentCurrent } from "./viewDocumentFreshness";

describe("isViewDocumentCurrent", () => {
  it("accepts matching revision and graph version", () => {
    expect(
      isViewDocumentCurrent("rev-1", 3, {
        revision: "rev-1",
        graphVersion: 3,
      }),
    ).toBe(true);
  });

  it("rejects stale revision", () => {
    expect(
      isViewDocumentCurrent("rev-2", 3, {
        revision: "rev-1",
        graphVersion: 3,
      }),
    ).toBe(false);
  });

  it("rejects stale graph version when revision matches", () => {
    expect(
      isViewDocumentCurrent("rev-1", 4, {
        revision: "rev-1",
        graphVersion: 2,
      }),
    ).toBe(false);
  });

  it("accepts legacy docs with no graph version", () => {
    expect(
      isViewDocumentCurrent("rev-1", 4, {
        revision: "rev-1",
      }),
    ).toBe(true);
  });
});
