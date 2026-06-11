import type {
  FirmwareGraph,
  NodeTypeDef,
  ValidationReport,
} from "../../features/PirAgent/types";
import type {
  PirAnalyzeResult,
  PirStatus,
  StructuralPatchRequest,
} from "../../features/PirAgent/pirTypes";

export type PirGraphView = "wiring" | "hld" | "lld" | "sequence";

export type PirGraphViewDocument = {
  schema_version: string;
  view: PirGraphView;
  revision: string;
  graph_version?: number;
  generated_at_ms: number;
  graph?: FirmwareGraph;
  mermaid?: string;
  title?: string;
  generation_error?: string;
};

function baseUrl(port: number): string {
  return `http://127.0.0.1:${port}/v1/pir-maker`;
}

async function handle<T>(r: Response): Promise<T> {
  if (!r.ok) {
    let detail = `HTTP ${r.status}`;
    try {
      const body = (await r.json()) as { detail?: string };
      if (body.detail) detail = body.detail;
    } catch {
      /* ignore */
    }
    throw new Error(detail);
  }
  return r.json() as Promise<T>;
}

async function waitForReady(port: number, chatId: string, timeoutMs = 120_000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const status = await pirStatus(port, chatId);
    if (status.status === "ready") {
      return;
    }
    if (status.status === "error") {
      throw new Error(status.error ?? "PIR analyze failed");
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error("PIR analyze timed out");
}

export async function pirFetchNodeRegistry(port: number): Promise<NodeTypeDef[]> {
  const r = await fetch(`${baseUrl(port)}/registry`);
  return handle<NodeTypeDef[]>(r);
}

export async function pirAnalyze(
  port: number,
  chatId: string,
  projectPath?: string,
  incremental = false,
  triggeredBy?: string,
  chatContext?: string,
): Promise<PirAnalyzeResult> {
  const r = await fetch(`${baseUrl(port)}/analyze`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      chat_id: chatId,
      project_path: projectPath,
      incremental,
      triggered_by: triggeredBy,
      async_mode: true,
      chat_context: chatContext?.trim() ? chatContext.trim() : undefined,
    }),
  });
  const body = (await r.json()) as PirAnalyzeResult | { status: string };
  if ("pir" in body) {
    return body;
  }
  if (body.status === "analyzing") {
    await waitForReady(port, chatId);
    const { result } = await pirDocument(port, chatId);
    return result;
  }
  if (!r.ok) {
    throw new Error(`HTTP ${r.status}`);
  }
  return body as PirAnalyzeResult;
}

export async function pirStatus(port: number, chatId: string): Promise<PirStatus> {
  const r = await fetch(
    `${baseUrl(port)}/status?chat_id=${encodeURIComponent(chatId)}`,
  );
  return handle<PirStatus>(r);
}

export async function pirDocument(
  port: number,
  chatId: string,
): Promise<{ result: PirAnalyzeResult }> {
  const r = await fetch(
    `${baseUrl(port)}/document?chat_id=${encodeURIComponent(chatId)}`,
  );
  return handle<{ result: PirAnalyzeResult }>(r);
}

export async function pirGraphViewDocument(
  port: number,
  chatId: string,
  view: PirGraphView,
): Promise<PirGraphViewDocument> {
  const r = await fetch(
    `${baseUrl(port)}/graph-view-document?chat_id=${encodeURIComponent(chatId)}&view=${encodeURIComponent(view)}`,
  );
  return handle<PirGraphViewDocument>(r);
}

export async function pirApplyNodePatch(
  port: number,
  chatId: string,
  nodeId: string,
  propertyUpdates: Record<string, unknown>,
  expectedRevision?: string,
): Promise<PirAnalyzeResult> {
  const r = await fetch(`${baseUrl(port)}/apply-node-patch`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      chat_id: chatId,
      node_id: nodeId,
      property_updates: propertyUpdates,
      expected_revision: expectedRevision,
    }),
  });
  return handle<PirAnalyzeResult>(r);
}

export async function pirApplyStructuralPatch(
  port: number,
  chatId: string,
  patch: StructuralPatchRequest,
  expectedRevision?: string,
): Promise<PirAnalyzeResult> {
  const r = await fetch(`${baseUrl(port)}/apply-structural-patch`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      chat_id: chatId,
      expected_revision: expectedRevision,
      ...patch,
    }),
  });
  return handle<PirAnalyzeResult>(r);
}

export async function pirApprove(
  port: number,
  chatId: string,
  comment?: string,
): Promise<void> {
  const r = await fetch(`${baseUrl(port)}/approve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ chat_id: chatId, comment }),
  });
  await handle<{ ok: boolean }>(r);
}

export async function pirWatch(
  port: number,
  chatId: string,
  projectPath: string,
): Promise<void> {
  const r = await fetch(`${baseUrl(port)}/watch`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ chat_id: chatId, project_path: projectPath }),
  });
  await handle<{ watching: boolean }>(r);
}

export async function pirUnwatch(port: number, chatId: string): Promise<void> {
  const r = await fetch(`${baseUrl(port)}/unwatch`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ chat_id: chatId }),
  });
  await handle<{ watching: boolean }>(r);
}

export type { FirmwareGraph, ValidationReport, NodeTypeDef };
