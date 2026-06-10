import { WORKSPACE_FILE_CONTENT_URL } from "./consts";

export type WorkspaceFileContentResponse = {
  path: string;
  content: string;
  truncated: boolean;
};

const contentCache = new Map<
  string,
  { content: string; truncated: boolean; at: number }
>();
const CACHE_TTL_MS = 60_000;

export function invalidateWorkspaceFileCache(filePath?: string): void {
  if (!filePath) {
    contentCache.clear();
    return;
  }
  contentCache.delete(filePath.trim());
}

export async function fetchWorkspaceFileContent(
  filePath: string,
  port = 8001,
  options?: { force?: boolean },
): Promise<WorkspaceFileContentResponse> {
  const path = filePath.trim();
  if (!path) {
    throw new Error("File path is empty");
  }

  const cacheKey = `${port}:${path}`;
  if (!options?.force) {
    const cached = contentCache.get(cacheKey);
    if (cached && Date.now() - cached.at < CACHE_TTL_MS) {
      return { path, content: cached.content, truncated: cached.truncated };
    }
  }

  const url = `http://127.0.0.1:${port}${WORKSPACE_FILE_CONTENT_URL}`;
  const response = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
    body: JSON.stringify({ path }),
  });

  if (!response.ok) {
    let detail = `HTTP ${response.status}`;
    try {
      const json = (await response.json()) as { detail?: string };
      if (typeof json.detail === "string") detail = json.detail;
    } catch {
      const text = await response.text();
      if (text) detail = text;
    }
    throw new Error(detail);
  }

  const data = (await response.json()) as WorkspaceFileContentResponse;
  contentCache.set(cacheKey, {
    content: data.content,
    truncated: data.truncated,
    at: Date.now(),
  });
  return data;
}
