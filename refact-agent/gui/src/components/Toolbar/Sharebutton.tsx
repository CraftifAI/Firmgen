import { useCallback, useRef, useState } from "react";
import { Box, IconButton } from "@radix-ui/themes";
import { FiShare } from "react-icons/fi";

import craftifPanelBtn from "../CraftifPanelButton/craftifPanelButton.module.css";
import { useConfig } from "../../hooks/useConfig";
import type { Config } from "../../features/Config/configSlice";
import {
  filenameFromContentDisposition,
  saveBlobToDisk,
} from "../../utils/saveBlobToDisk";

function buildV1RequestUrl(pathAndQuery: string, config: Config): string {
  const path = pathAndQuery.startsWith("/") ? pathAndQuery : `/${pathAndQuery}`;
  if (import.meta.env.DEV) {
    return path;
  }
  const base = (
    config.lspUrl?.trim() || `http://127.0.0.1:${config.lspPort}`
  ).replace(/\/$/, "");
  return `${base}${path}`;
}

export type SharebuttonProps = {
  chatId: string;
};

export const Sharebutton = ({ chatId }: SharebuttonProps) => {
  const config = useConfig();
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);

  const handleShare = useCallback(async () => {
    if (!chatId || inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    try {
      const url = buildV1RequestUrl(
        `/v1/esp32/factory-release?chat_id=${encodeURIComponent(chatId)}`,
        config,
      );
      const res = await fetch(url);
      const contentType = res.headers.get("Content-Type") || "";

      if (!res.ok) {
        const errText = await res.text().catch(() => res.statusText);
        window.alert(
          errText.trim() || `Could not download factory release (${res.status}).`,
        );
        return;
      }

      const looksBinary =
        contentType.includes("zip") ||
        contentType.includes("octet-stream") ||
        contentType.includes("application/x-zip");

      if (!looksBinary) {
        const errText = await res.text();
        window.alert(
          errText.trim() || "Server returned an unexpected response (not a ZIP).",
        );
        return;
      }

      const blob = await res.blob();
      const suggestedName = filenameFromContentDisposition(
        res.headers.get("Content-Disposition"),
        `esp32-factory-${chatId.slice(0, 8)}.zip`,
      );

      await saveBlobToDisk(blob, {
        suggestedName,
        fileTypes: [
          { description: "ZIP archive", accept: { "application/zip": [".zip"] } },
        ],
      });
    } catch (e) {
      const hint =
        import.meta.env.DEV
          ? " Is the agent running and REFACT_LSP_URL (or the Vite /v1 proxy target) correct?"
          : " Is the agent running and lspUrl / port correct?";
      window.alert((e instanceof Error ? e.message : "Download failed.") + hint);
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  }, [chatId, config]);

  const disabled = !chatId || busy;

  return (
    <Box
      style={{
        display: "inline-flex",
        alignItems: "center",
        transform: "translateY(-4px)",
        marginBottom: "5px",
      }}
    >
      <IconButton
        type="button"
        size="2"
        variant="ghost"
        title={
          disabled && !chatId
            ? "Share (open a chat first)"
            : busy
              ? "Preparing download..."
              : "Export Firmware Package"
        }
        aria-label="Share"
        aria-busy={busy}
        disabled={disabled}
        className={craftifPanelBtn.settingsTrigger + " " + craftifPanelBtn.hoverable}
        onClick={() => {
          void handleShare();
        }}
      >
        <FiShare size={16} aria-hidden />
      </IconButton>
    </Box>
  );
};

export default Sharebutton;
