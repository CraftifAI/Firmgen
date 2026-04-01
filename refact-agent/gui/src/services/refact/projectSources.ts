function sourcesUrl(port: number): string {
  return `http://127.0.0.1:${port}/v1/esp32/project-sources`;
}

/** Opens `<project_root>/sources` in the system file manager (via local engine). */
export async function openProjectSourcesInFileManager(
  projectRoot: string,
  port = 8001,
): Promise<{ opened: string }> {
  const root = projectRoot.trim();
  if (!root) {
    throw new Error("Project path is empty");
  }
  const r = await fetch(`${sourcesUrl(port)}/open`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
    body: JSON.stringify({ project_root: root }),
  });
  if (!r.ok) {
    let detail = `HTTP ${r.status}`;
    try {
      const j = (await r.json()) as { detail?: string };
      if (typeof j.detail === "string") detail = j.detail;
    } catch {
      const t = await r.text();
      if (t) detail = t;
    }
    throw new Error(detail);
  }
  return r.json() as Promise<{ opened: string }>;
}

export type ProjectSourceFile = {
  name: string;
  size_bytes: number;
};

export type ProjectSourcesListResult = {
  directory: string;
  files: ProjectSourceFile[];
};

export async function listProjectSources(
  projectRoot: string,
  port = 8001,
): Promise<ProjectSourcesListResult> {
  const u = new URL(sourcesUrl(port));
  u.searchParams.set("project_root", projectRoot.trim());
  const r = await fetch(u.toString(), { credentials: "same-origin" });
  if (!r.ok) {
    let detail = `HTTP ${r.status}`;
    try {
      const j = (await r.json()) as { detail?: string };
      if (typeof j.detail === "string") detail = j.detail;
    } catch {
      const t = await r.text();
      if (t) detail = t;
    }
    throw new Error(detail);
  }
  return r.json() as Promise<ProjectSourcesListResult>;
}

export async function uploadProjectSources(
  projectRoot: string,
  files: File[],
  port = 8001,
): Promise<{ saved: string[]; directory: string }> {
  if (files.length === 0) {
    throw new Error("Select at least one file");
  }
  const fd = new FormData();
  fd.append("project_root", projectRoot.trim());
  for (const f of files) {
    fd.append("file", f, f.name);
  }
  const r = await fetch(sourcesUrl(port), {
    method: "POST",
    body: fd,
    credentials: "same-origin",
  });
  if (!r.ok) {
    let detail = `HTTP ${r.status}`;
    try {
      const j = (await r.json()) as { detail?: string };
      if (typeof j.detail === "string") detail = j.detail;
    } catch {
      const t = await r.text();
      if (t) detail = t;
    }
    throw new Error(detail);
  }
  return r.json() as Promise<{ saved: string[]; directory: string }>;
}

/** Convert a data URL (e.g. pasted image) into a File for upload. */
export function dataUrlToFile(
  dataUrl: string,
  filename: string,
  fallbackMime: string,
): File | null {
  if (!dataUrl.startsWith("data:")) return null;
  const comma = dataUrl.indexOf(",");
  if (comma < 0) return null;
  const header = dataUrl.slice(0, comma);
  const b64 = dataUrl.slice(comma + 1);
  const mimeMatch = /^data:([^;]+);/i.exec(header);
  const mime =
    mimeMatch?.[1]?.trim() || fallbackMime || "application/octet-stream";
  try {
    const binary = atob(b64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return new File([bytes], filename, { type: mime });
  } catch {
    return null;
  }
}

/** Avoid overwriting when archiving multiple files with the same name. */
export function uniqueArchiveFilename(originalName: string, index: number): string {
  const t = Date.now();
  const clean =
    (originalName || "file").replace(/[/\\]/g, "_").slice(0, 120) || "file";
  const dot = clean.lastIndexOf(".");
  if (dot <= 0) return `${clean}_${t}_${index}`;
  return `${clean.slice(0, dot)}_${t}_${index}${clean.slice(dot)}`;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export { formatBytes };
