/**
 * Craftif cloud API base for auth/usage/admin routes.
 * - Empty string: same-origin (Vite dev server proxies /auth and /usage).
 * - Absolute URL: Electron packaged app (vite.app.config.ts default).
 */
export function craftifApiBaseUrl(): string {
  const raw = import.meta.env.VITE_CRAFTIF_API_BASE as string | undefined;
  if (raw == null || raw === "") return "";
  return raw.replace(/\/+$/, "");
}

/** Build URL for Craftif REST paths (e.g. "/auth/login"). */
export function craftifApiUrl(path: string): string {
  const base = craftifApiBaseUrl();
  const p = path.startsWith("/") ? path : `/${path}`;
  if (!base) return p;
  return `${base}${p}`;
}
