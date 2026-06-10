import type { ChatMessages } from "./types";
import {
  isAssistantMessage,
  isToolMessage,
  type ToolResult,
} from "./types";

function projectTreeUrl(port: number): string {
  return `http://127.0.0.1:${port}/v1/esp32/project-tree`;
}

export type ProjectTreeNode = {
  name: string;
  path: string;
  type: "file" | "dir";
  size_bytes?: number;
  children?: ProjectTreeNode[];
};

export type ProjectTreeListResult = {
  root: string;
  tree: ProjectTreeNode[];
};

const TREE_CACHE_TTL_MS = 500;
const treeCache = new Map<
  string,
  { at: number; result: ProjectTreeListResult }
>();

export function invalidateProjectTreeCache(
  projectRoot?: string,
  port = 8001,
  maxDepth = 8,
): void {
  if (!projectRoot?.trim()) {
    treeCache.clear();
    return;
  }
  treeCache.delete(`${port}:${projectRoot.trim()}:${maxDepth}`);
}

export type ChatProjectPathResult = {
  project_path: string | null;
};

export async function fetchChatEsp32ProjectPath(
  chatId: string,
  port = 8001,
): Promise<string | null> {
  const id = chatId.trim();
  if (!id) return null;
  const u = new URL(`http://127.0.0.1:${port}/v1/esp32/chat-project-path`);
  u.searchParams.set("chat_id", id);
  const r = await fetch(u.toString(), { credentials: "same-origin" });
  if (!r.ok) {
    return null;
  }
  const j = (await r.json()) as ChatProjectPathResult;
  return j.project_path?.trim() || null;
}

export async function fetchProjectTree(
  projectRoot: string,
  port = 8001,
  maxDepth = 8,
  options?: { force?: boolean },
): Promise<ProjectTreeListResult> {
  const root = projectRoot.trim();
  if (!root) {
    throw new Error("Project path is empty");
  }
  const cacheKey = `${port}:${root}:${maxDepth}`;
  const force = options?.force ?? false;
  if (force) {
    invalidateProjectTreeCache(root, port, maxDepth);
  } else {
    const cached = treeCache.get(cacheKey);
    if (cached && Date.now() - cached.at < TREE_CACHE_TTL_MS) {
      return cached.result;
    }
  }
  const u = new URL(projectTreeUrl(port));
  u.searchParams.set("project_root", root);
  u.searchParams.set("max_depth", String(maxDepth));
  if (force) {
    u.searchParams.set("_t", String(Date.now()));
  }
  const r = await fetch(u.toString(), {
    credentials: "same-origin",
    cache: force ? "no-store" : "default",
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
  const result = (await r.json()) as ProjectTreeListResult;
  treeCache.set(cacheKey, { at: Date.now(), result });
  return result;
}

export async function openProjectTreeFile(
  projectRoot: string,
  filePath: string,
  port = 8001,
): Promise<{ opened: string }> {
  const root = projectRoot.trim();
  const file = filePath.trim();
  if (!root) throw new Error("Project path is empty");
  if (!file) throw new Error("File path is empty");
  const r = await fetch(`${projectTreeUrl(port)}/open`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
    body: JSON.stringify({ project_root: root, file_path: file }),
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

const PROJECT_PATH_REGEXES = [
  /Project path:\s*(.+?)(?:\r?\n|$)/i,
  /Project\s+['"](.+?)['"]\s+is\s+valid/i,
  /using existing project\s+['"].+?['"]\s+at\s+(.+?)(?:\r?\n|$)/i,
  /at\s+([a-zA-Z]:[\\/][^'\r\n]+)/i,
];

function toolMessageText(result: ToolResult): string {
  if (typeof result.content === "string") return result.content;
  return "";
}

/** Parse the latest agent-created/validated ESP-IDF path from chat tool messages. */
export function extractEsp32ProjectPathFromMessages(
  messages: ChatMessages,
): string | null {
  let latestPath: string | null = null;

  for (const msg of messages) {
    // 1. Check assistant messages for tool calls with `project_path` argument
    if (isAssistantMessage(msg) && msg.tool_calls) {
      for (const tc of msg.tool_calls) {
        if (tc.function.name?.startsWith("esp32_")) {
          try {
            const args = JSON.parse(tc.function.arguments) as {
              project_path?: string;
            };
            if (args.project_path?.trim()) {
              latestPath = args.project_path.trim();
            }
          } catch {
            // ignore
          }
        }
      }
    }

    // 2. Check tool messages for path pattern matches in their text content
    if (isToolMessage(msg)) {
      const text = toolMessageText(msg.content);
      for (const rx of PROJECT_PATH_REGEXES) {
        const match = rx.exec(text);
        if (match?.[1]) {
          const matchedPath = match[1].trim();
          if (matchedPath) {
            latestPath = matchedPath;
          }
        }
      }
    }
  }

  return latestPath;
}

/** Resolve the on-disk ESP-IDF project folder for the current chat sidebar tree. */
export function resolveActiveEsp32ProjectPath(args: {
  chatId?: string;
  messages: ChatMessages;
  progressProjectPath?: string | null;
  apiProjectPath?: string | null;
  threadProjectPath?: string | null;
}): string {
  const fromThread = args.threadProjectPath?.trim();
  if (fromThread) return fromThread;

  const fromApi = args.apiProjectPath?.trim();
  if (fromApi) return fromApi;

  const fromProgress = args.progressProjectPath?.trim();
  if (fromProgress) return fromProgress;

  const fromMessages = extractEsp32ProjectPathFromMessages(args.messages);
  if (fromMessages) return fromMessages;

  return "";
}
