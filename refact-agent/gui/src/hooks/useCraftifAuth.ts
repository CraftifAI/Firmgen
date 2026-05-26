import { useState, useCallback, useSyncExternalStore, useEffect } from "react";
import { CRAFTIF_API_BASE } from "../config/craftifApiBase";

const DISPLAY_NAME_CHANGE_EVENT = "craftif-display-name-changed";
const SESSION_JWT_STORAGE_KEY = "craftif_session_jwt";

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

function clearStoredSessionJwt(): void {
    try {
        if (typeof window !== "undefined") {
            localStorage.removeItem(SESSION_JWT_STORAGE_KEY);
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

function persistSessionJwt(token: string): void {
    try {
        if (typeof window !== "undefined") {
            localStorage.setItem(SESSION_JWT_STORAGE_KEY, token);
        }
    } catch {
        /* ignore */
    }
}

function readStoredSessionJwt(): string | null {
    if (typeof window === "undefined") return null;
    try {
        const token = localStorage.getItem(SESSION_JWT_STORAGE_KEY);
        if (!token) return null;
        if (isJwtExpired(token)) {
            clearStoredSessionJwt();
            return null;
        }
        return token;
    } catch {
        return null;
    }
}

export let sessionJwt: string | null = readStoredSessionJwt();

const DISPLAY_NAME_STORAGE_KEY = "craftif_display_name";

/** In-memory mirror; sessionStorage is source of truth after login (survives navigation). */
export let sessionDisplayName: string | null = null;

function readStoredDisplayName(): string | null {
    if (typeof window === "undefined") return sessionDisplayName;
    try {
        return sessionStorage.getItem(DISPLAY_NAME_STORAGE_KEY) ?? sessionDisplayName;
    } catch {
        return sessionDisplayName;
    }
}

export function clearStoredCraftifDisplayName(): void {
    sessionDisplayName = null;
    try {
        if (typeof window !== "undefined") {
            sessionStorage.removeItem(DISPLAY_NAME_STORAGE_KEY);
        }
    } catch {
        /* ignore quota / private mode */
    }
    notifyDisplayNameChanged();
}

function persistDisplayName(name: string): void {
    sessionDisplayName = name;
    try {
        if (typeof window !== "undefined") {
            sessionStorage.setItem(DISPLAY_NAME_STORAGE_KEY, name);
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

    const login = useCallback(async (email: string, password: string, displayNameHint?: string) => {
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
            persistSessionJwt(data.token);

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
                displayNameHint,
                data.username,
                email,
            );
            persistDisplayName(resolvedName);

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
