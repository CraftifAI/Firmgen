export type ViewSnapshot = {
  revision?: string;
  graphVersion?: number;
};

export function isViewDocumentCurrent(
  currentRevision: string | null,
  currentGraphVersion: number | null,
  doc?: ViewSnapshot | null,
): boolean {
  if (!doc?.revision) return false;
  if (currentRevision && doc.revision !== currentRevision) return false;
  if (
    currentGraphVersion != null &&
    doc.graphVersion != null &&
    doc.graphVersion !== currentGraphVersion
  ) {
    return false;
  }
  return true;
}
