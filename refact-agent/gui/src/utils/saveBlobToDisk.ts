export type SaveBlobOptions = {
  suggestedName: string;
  fileTypes?: Array<{ description: string; accept: Record<string, string[]> }>;
};

function isAbortError(e: unknown): boolean {
  return e instanceof DOMException && e.name === "AbortError";
}

export async function saveBlobToDisk(
  blob: Blob,
  { suggestedName, fileTypes }: SaveBlobOptions,
): Promise<void> {
  if (typeof window === "undefined") return;

  const picker = (window as Window & { showSaveFilePicker?: (opts: unknown) => Promise<{
    createWritable: () => Promise<{ write: (b: Blob) => Promise<void>; close: () => Promise<void> }>;
  }> }).showSaveFilePicker;

  if (typeof picker === "function") {
    try {
      const types =
        fileTypes && fileTypes.length > 0
          ? fileTypes
          : [{ description: "All files", accept: { "*/*": [".zip", ".json", ".bin"] } }];

      const handle = await picker({ suggestedName, types });
      const writable = await handle.createWritable();
      await writable.write(blob);
      await writable.close();
      return;
    } catch (e) {
      if (isAbortError(e)) return;
    }
  }

  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = suggestedName;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

export function filenameFromContentDisposition(
  header: string | null,
  fallback: string,
): string {
  if (!header) return fallback;
  const utf8 = /filename\*\s*=\s*UTF-8''([^;]+)/i.exec(header);
  if (utf8?.[1]) {
    try {
      return decodeURIComponent(utf8[1].trim());
    } catch {
      return utf8[1].trim();
    }
  }
  const quoted = /filename\s*=\s*"([^"]+)"/i.exec(header);
  if (quoted?.[1]) return quoted[1].trim();
  const plain = /filename\s*=\s*([^;]+)/i.exec(header);
  if (plain?.[1]) return plain[1].trim().replace(/^["']|["']$/g, "");
  return fallback;
}
