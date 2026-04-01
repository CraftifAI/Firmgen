/**
 * fileParser.ts
 *
 * Frontend service for uploading and parsing files via the backend
 * `/v1/upload` endpoint. Replaces heavy client-side parsing libs
 * (pdfjs-dist, mammoth, etc.) with a lean server-side approach.
 */

// ── Types ─────────────────────────────────────────────────────────────────────

export interface ParsedFile {
    /** Original upload filename */
    filename: string;
    /** Resolved MIME type (set by the server) */
    mime_type: string;
    /** Raw file size in bytes */
    size_bytes: number;
    /** Extracted plain-text content */
    text: string;
    /** Format-specific metadata (e.g. page_count, slide_count, sheet_names) */
    metadata: Record<string, unknown>;
}

export interface SupportedFormats {
    extensions: string[];
    mime_types: string[];
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * Normalise an API base URL by stripping trailing slashes.
 */
function normalise(apiBase: string): string {
    return apiBase.replace(/\/+$/, "");
}

// ── Main API ──────────────────────────────────────────────────────────────────

/**
 * Upload a single file to the backend and return its parsed contents.
 *
 * @param file     The File object from the browser.
 * @param apiBase  Root URL of the refactapi server, e.g. "http://localhost:8002".
 * @returns        Structured `ParsedFile` response from the server.
 * @throws         An Error with a human-readable message on failure.
 *
 * @example
 * const result = await parseFile(myFile, "http://localhost:8002");
 * console.log(result.text.slice(0, 200));
 */
export async function parseFile(
    file: File,
    apiBase: string,
): Promise<ParsedFile> {
    const form = new FormData();
    form.append("file", file);

    const url = `${normalise(apiBase)}/v1/upload`;
    let response: Response;

    try {
        response = await fetch(url, {
            method: "POST",
            body: form,
        });
    } catch (networkError) {
        throw new Error(
            `Network error while uploading "${file.name}": ${String(networkError)}`,
        );
    }

    if (!response.ok) {
        let detail = `HTTP ${response.status}`;
        try {
            const body = (await response.json()) as { detail?: string };
            if (body.detail) detail = body.detail;
        } catch {
            // body was not JSON — ignore
        }
        throw new Error(`Upload failed for "${file.name}": ${detail}`);
    }

    return (await response.json()) as ParsedFile;
}

/**
 * Fetch the list of file extensions and MIME types supported by the backend.
 *
 * @param apiBase  Root URL of the refactapi server.
 */
export async function fetchSupportedFormats(
    apiBase: string,
): Promise<SupportedFormats> {
    const url = `${normalise(apiBase)}/v1/upload/supported-formats`;
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`Failed to fetch supported formats: HTTP ${response.status}`);
    }
    return (await response.json()) as SupportedFormats;
}

/**
 * Upload multiple files in parallel and return results.
 * Failed uploads are collected in the `errors` array rather than throwing.
 *
 * @param files    Array of File objects.
 * @param apiBase  Root URL of the refactapi server.
 */
export async function parseFiles(
    files: File[],
    apiBase: string,
): Promise<{ results: ParsedFile[]; errors: { file: string; error: string }[] }> {
    const results: ParsedFile[] = [];
    const errors: { file: string; error: string }[] = [];

    await Promise.all(
        files.map(async (file) => {
            try {
                const parsed = await parseFile(file, apiBase);
                results.push(parsed);
            } catch (err) {
                errors.push({
                    file: file.name,
                    error: err instanceof Error ? err.message : String(err),
                });
            }
        }),
    );

    return { results, errors };
}
