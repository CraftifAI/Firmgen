import { useState, useCallback, useSyncExternalStore, useEffect } from "react";
import { CRAFTIF_API_BASE } from "../config/craftifApiBase";

const DISPLAY_NAME_CHANGE_EVENT = "craftif-display-name-changed";
const SESSION_JWT_STORAGE_KEY = "craftif_session_jwt";
const DISPLAY_NAME_STORAGE_KEY = "craftif_display_name";
export const REMEMBER_ME_PREF_KEY = "craftif_remember_me";

function notifyDisplayNameChanged() {
    if (typeof window !== "undefined") {
        window.dispatchEvent(new Event(DISPLAY_NAME_CHANGE_EVENT));
    }
}

function subscribeDisplayName(onStoreChange: () => void) {
    if (typeof window === "undefined") return () => {};
    window.addEventListener(DISPLAY_NAME_CHANGE_EVENT, onStoreChange);
    return () => window.removeEventListener(DISPLAY_NAME_CHANGE_EVENT, onStoreChange);
}

export interface User {
    id: string;
    email: string;
    role?: string;
    username?: string;
}

export interface CraftifLoginOptions {
    displayNameHint?: string;
    rememberMe?: boolean;
}

function decodeJwtPayload(token: string): Record<string, unknown> | null {
    const parts = token.split(".");
    if (parts.length < 2) return null;
    try {
        const b64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
        const padded = b64 + "=".repeat((4 - (b64.length % 4 || 4)) % 4);
        const decoded = atob(padded);
        return JSON.parse(decoded) as Record<string, unknown>;
    } catch {
        return null;
    }
}

function isJwtExpired(token: string): boolean {
    const payload = decodeJwtPayload(token);
    const exp = payload?.exp;
    if (typeof exp !== "number") return false;
    const nowInSeconds = Math.floor(Date.now() / 1000);
    return exp <= nowInSeconds;
}

function displayNameFromJwt(token: string): string | null {
    const payload = decodeJwtPayload(token);
    if (!payload) return null;

    const candidates = [
        payload.username,
        payload.name,
        payload.displayName,
        payload.preferred_username,
    ];
    for (const candidate of candidates) {
        if (typeof candidate === "string" && candidate.trim()) {
            return candidate.trim();
        }
    }

    if (typeof payload.email === "string" && payload.email.includes("@")) {
        const local = payload.email.split("@")[0]?.trim();
        if (local) return local;
    }

    return null;
}

function clearStoredSessionJwt(): void {
    try {
        if (typeof window !== "undefined") {
            localStorage.removeItem(SESSION_JWT_STORAGE_KEY);
            sessionStorage.removeItem(SESSION_JWT_STORAGE_KEY);
        }
    } catch {
        /* ignore */
    }
}

/** Clears in-memory + persisted Craftif JWT (shared with useLogout). */
export function clearCraftifSessionJwt(): void {
    sessionJwt = null;
    clearStoredSessionJwt();
}

function persistSessionJwt(token: string, rememberMe: boolean): void {
    try {
        if (typeof window === "undefined") return;
        if (rememberMe) {
            localStorage.setItem(SESSION_JWT_STORAGE_KEY, token);
            sessionStorage.removeItem(SESSION_JWT_STORAGE_KEY);
        } else {
            sessionStorage.setItem(SESSION_JWT_STORAGE_KEY, token);
            localStorage.removeItem(SESSION_JWT_STORAGE_KEY);
        }
    } catch {
        /* ignore */
    }
}

function readStoredSessionJwt(): string | null {
    if (typeof window === "undefined") return null;
    try {
        for (const storage of [localStorage, sessionStorage]) {
            const token = storage.getItem(SESSION_JWT_STORAGE_KEY);
            if (!token) continue;
            if (isJwtExpired(token)) {
                storage.removeItem(SESSION_JWT_STORAGE_KEY);
                continue;
            }
            return token;
        }
        return null;
    } catch {
        return null;
    }
}

/** In-memory mirror; storage is source of truth after login. */
export let sessionDisplayName: string | null = null;

export let sessionJwt: string | null = readStoredSessionJwt();

function readStoredDisplayName(): string | null {
    if (typeof window === "undefined") return sessionDisplayName;
    try {
        return (
            localStorage.getItem(DISPLAY_NAME_STORAGE_KEY) ??
            sessionStorage.getItem(DISPLAY_NAME_STORAGE_KEY) ??
            sessionDisplayName
        );
    } catch {
        return sessionDisplayName;
    }
}

function hydrateDisplayNameFromStorage(token: string | null): void {
    if (!token) return;

    const stored = readStoredDisplayName();
    if (stored?.trim()) {
        sessionDisplayName = stored.trim();
        return;
    }

    const fromJwt = displayNameFromJwt(token);
    if (!fromJwt) return;

    sessionDisplayName = fromJwt;
    try {
        if (typeof window === "undefined") return;
        if (localStorage.getItem(SESSION_JWT_STORAGE_KEY)) {
            localStorage.setItem(DISPLAY_NAME_STORAGE_KEY, fromJwt);
        } else if (sessionStorage.getItem(SESSION_JWT_STORAGE_KEY)) {
            sessionStorage.setItem(DISPLAY_NAME_STORAGE_KEY, fromJwt);
        }
    } catch {
        /* ignore */
    }
}

hydrateDisplayNameFromStorage(sessionJwt);

export function readRememberMePreference(): boolean {
    if (typeof window === "undefined") return false;
    try {
        return localStorage.getItem(REMEMBER_ME_PREF_KEY) === "true";
    } catch {
        return false;
    }
}

function persistRememberMePreference(rememberMe: boolean): void {
    try {
        if (typeof window !== "undefined") {
            localStorage.setItem(REMEMBER_ME_PREF_KEY, rememberMe ? "true" : "false");
        }
    } catch {
        /* ignore */
    }
}

export function clearStoredCraftifDisplayName(): void {
    sessionDisplayName = null;
    try {
        if (typeof window !== "undefined") {
            localStorage.removeItem(DISPLAY_NAME_STORAGE_KEY);
            sessionStorage.removeItem(DISPLAY_NAME_STORAGE_KEY);
        }
    } catch {
        /* ignore quota / private mode */
    }
    notifyDisplayNameChanged();
}

function persistDisplayName(name: string, rememberMe: boolean): void {
    sessionDisplayName = name;
    try {
        if (typeof window !== "undefined") {
            if (rememberMe) {
                localStorage.setItem(DISPLAY_NAME_STORAGE_KEY, name);
                sessionStorage.removeItem(DISPLAY_NAME_STORAGE_KEY);
            } else {
                sessionStorage.setItem(DISPLAY_NAME_STORAGE_KEY, name);
                localStorage.removeItem(DISPLAY_NAME_STORAGE_KEY);
            }
        }
    } catch {
        /* ignore */
    }
    notifyDisplayNameChanged();
}

function resolveDisplayName(
    hint: string | undefined,
    apiUsername: unknown,
    email: string,
): string {
    const fromHint = hint?.trim();
    if (fromHint) return fromHint;
    if (typeof apiUsername === "string" && apiUsername.trim()) {
        return apiUsername.trim();
    }
    const local = email.split("@")[0]?.trim();
    if (local) return local;
    return "User";
}

export const useCraftifAuth = () => {
    const [jwt, setJwt] = useState<string | null>(sessionJwt);
    const [user, setUser] = useState<User | null>(null);
    const displayName = useSyncExternalStore(
        subscribeDisplayName,
        readStoredDisplayName,
        () => null,
    );
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (!sessionJwt) return;
        void fetch("http://127.0.0.1:8002/v1/proxy-set-jwt", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ token: sessionJwt }),
        }).catch(() => {
            /* best-effort during app startup */
        });
    }, []);

    const login = useCallback(async (
        email: string,
        password: string,
        options?: CraftifLoginOptions,
    ) => {
        const rememberMe = options?.rememberMe ?? false;
        setLoading(true);
        setError(null);
        try {
            const resp = await fetch(`${CRAFTIF_API_BASE}/auth/login`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, password }),
            });

            if (!resp.ok) {
                const errorData = await resp.json().catch(() => ({}));
                throw new Error(errorData.message || "Invalid credentials.");
            }

            const data = await resp.json();
            if (!data.token) {
                throw new Error("No token returned from server.");
            }

            setJwt(data.token);
            sessionJwt = data.token;
            persistSessionJwt(data.token, rememberMe);
            persistRememberMePreference(rememberMe);

            // Push the JWT out-of-band directly to the local python proxy.
            // This bypasses the Rust agent's reliance on `cli.yaml` dummy keys.
            try {
                await fetch("http://127.0.0.1:8002/v1/proxy-set-jwt", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ token: data.token })
                });
            } catch (proxyErr) {
                console.error("Failed to arm local Python proxy with JWT", proxyErr);
            }

            const resolvedName = resolveDisplayName(
                options?.displayNameHint,
                data.username,
                email,
            );
            persistDisplayName(resolvedName, rememberMe);

            setUser({
                id: data.id || "user",
                email: email,
                role: data.role || "USER",
                username: resolvedName,
            });

            return data.token as string;
        } catch (err: unknown) {
            const message = err instanceof Error ? err.message : "An unknown error occurred during login.";
            setError(message);
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const register = useCallback(async (email: string, password: string) => {
        setLoading(true);
        setError(null);
        try {
            const resp = await fetch(`${CRAFTIF_API_BASE}/auth/register`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, password }),
            });

            if (!resp.ok) {
                const errorData = await resp.json().catch(() => ({}));
                throw new Error(errorData.error || errorData.message || "Registration failed.");
            }

            const data = await resp.json();
            return data;
        } catch (err: unknown) {
            const message = err instanceof Error ? err.message : "An unknown error occurred during registration.";
            setError(message);
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    /**
     * Step 1 of FirmGen OTP signup: sends a 6-digit OTP to the user's email.
     * POST /auth/send-signup-otp  { email, password }
     */
    const sendSignupOtp = useCallback(async (email: string, password: string) => {
        setLoading(true);
        setError(null);
        try {
            const resp = await fetch(`${CRAFTIF_API_BASE}/auth/send-signup-otp`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, password }),
            });

            if (!resp.ok) {
                const errorData = await resp.json().catch(() => ({}));
                throw new Error(errorData.error || errorData.message || "Failed to send verification code.");
            }

            return await resp.json() as { message: string };
        } catch (err: unknown) {
            const message = err instanceof Error ? err.message : "An unknown error occurred.";
            setError(message);
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    /**
     * Step 2 of FirmGen OTP signup: verifies the OTP and creates the user account.
     * POST /auth/verify-signup-otp  { email, otp, appSource: "FirmGen" }
     */
    const verifySignupOtp = useCallback(async (email: string, otp: string) => {
        setLoading(true);
        setError(null);
        try {
            const resp = await fetch(`${CRAFTIF_API_BASE}/auth/verify-signup-otp`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, otp, appSource: "FirmGen" }),
            });

            if (!resp.ok) {
                const errorData = await resp.json().catch(() => ({}));
                throw new Error(errorData.error || errorData.message || "OTP verification failed.");
            }

            return await resp.json() as { id: string; email: string };
        } catch (err: unknown) {
            const message = err instanceof Error ? err.message : "An unknown error occurred.";
            setError(message);
            throw err;
        } finally {
            setLoading(false);
        }
    }, []);

    const logout = useCallback(() => {
        setJwt(null);
        clearCraftifSessionJwt();
        setUser(null);
        clearStoredCraftifDisplayName();
        setError(null);
    }, []);

    return {
        jwt,
        user,
        displayName,
        loading,
        error,
        login,
        register,
        sendSignupOtp,
        verifySignupOtp,
        logout,
    };
};
