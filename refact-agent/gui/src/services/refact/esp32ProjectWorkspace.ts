const LS_KEY = "craftif_esp_workspace_parent";

/** Last-used parent directory for "New ESP project folder" (absolute path). */
export function getDefaultEspWorkspaceParent(): string {
  if (typeof localStorage === "undefined") return "";
  return localStorage.getItem(LS_KEY) ?? "";
}

export function setDefaultEspWorkspaceParent(path: string): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(LS_KEY, path);
}

function parentDirFromFileOrFolder(p: string): string {
  const s = p.replace(/\\/g, "/").replace(/\/+$/, "");
  const i = s.lastIndexOf("/");
  if (i <= 0) return s;
  return s.slice(0, i);
}

/** Suggested on-disk folder name from a display title (ESP32 path rules). */
export function slugifyEspWorkspaceFolderName(displayName: string): string {
  const s = displayName
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 64);
  return s || "project";
}

export function defaultEspWorkspaceParentCandidates(integrationPath?: string): string {
  const fromIntegration =
    integrationPath && integrationPath.trim().length > 0
      ? parentDirFromFileOrFolder(integrationPath.trim())
      : "";
  return getDefaultEspWorkspaceParent() || fromIntegration;
}

export async function createEsp32ProjectWorkspace(args: {
  parentPath: string;
  folderName: string;
  port?: number;
}): Promise<{ path: string }> {
  const port = args.port ?? 8001;
  const r = await fetch(
    `http://127.0.0.1:${port}/v1/esp32/create-project-workspace`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        parent_path: args.parentPath.trim(),
        folder_name: args.folderName.trim(),
      }),
    },
  );
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
  return r.json() as Promise<{ path: string }>;
}
