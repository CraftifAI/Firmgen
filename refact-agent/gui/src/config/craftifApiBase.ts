/**
 * Absolute base URL for Craftif / Orbit cloud API (auth, admin, etc.).
 * Must not be relative: Electron loads the UI from app:// so fetch("/…") would target the wrong origin.
 */
export const CRAFTIF_API_BASE = (
    import.meta.env.VITE_CRAFTIF_API_BASE || "https://api.craftifai.com"
).replace(/\/+$/, "");
