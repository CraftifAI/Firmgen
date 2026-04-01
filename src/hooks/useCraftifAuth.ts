import { useState, useCallback, useSyncExternalStore } from "react";
import { craftifApiUrl } from "../utils/craftifApi";

const DISPLAY_NAME_CHANGE_EVENT = "craftif-display-name-changed";

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

export let sessionJwt: string | null = null;

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

    const login = useCallback(async (email: string, password: string, displayNameHint?: string) => {
        setLoading(true);
        setError(null);
        try {
            const resp = await fetch(craftifApiUrl("/auth/login"), {
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
            const resp = await fetch(craftifApiUrl("/auth/register"), {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, password }),
            });

            if (!resp.ok) {
                const errorData = await resp.json().catch(() => ({}));
                throw new Error(errorData.message || "Registration failed.");
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

    const logout = useCallback(() => {
        setJwt(null);
        sessionJwt = null;
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
        logout,
    };
};
