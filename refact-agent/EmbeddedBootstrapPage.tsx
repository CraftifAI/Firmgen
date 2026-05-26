import React, { useState, useCallback, useEffect } from "react";
import {
  Card,
  Flex,
  Heading,
  Text,
  TextField,
  Button,
  Box,
  Separator,
} from "@radix-ui/themes";

// ─── Desktop bridge ────────────────────────────────────────────────────────────
type CraftifDesktopBridge = {
  getSettings?: () => Promise<Record<string, unknown>>;
  saveWizardSettings?: (data: WizardSettings) => Promise<{ ok: boolean; error?: string }>;
  browseFolder?: () => Promise<string | null>;
  browseFile?: (opts?: { title?: string; filters?: { name: string; extensions: string[] }[] }) => Promise<string | null>;
};

function getDesktopBridge(): CraftifDesktopBridge | undefined {
  if (typeof window === "undefined") return undefined;
  return (window as Window & { craftifai?: CraftifDesktopBridge }).craftifai;
}

// ─── Types ─────────────────────────────────────────────────────────────────────
interface WizardSettings {
  idf_export_sh: string | null;
  board_definition: string;
  project_dir: string | null;
}

const BOARDS = [
  { id: "esp32-s3-DevKitM-1-N16R8",        name: "ESP32-S3 DevKitM-1",   desc: "N16R8 · 16 MB flash · 8 MB PSRAM" },
  { id: "esp32-s3-devkitc-1-n32r8v",        name: "ESP32-S3 DevKitC-1",   desc: "N32R8V · 32 MB flash · 8 MB PSRAM" },
  { id: "esp32-wroom-32",                    name: "ESP32-WROOM-32",        desc: "Classic · 4 MB flash" },
  { id: "ESP32-S3-WROOM-1-N16R8-touch-lcd-4b", name: "ESP32-S3 Touch LCD", desc: "N16R8 · 4\" touch display" },
] as const;

// ─── Props ─────────────────────────────────────────────────────────────────────
type EmbeddedBootstrapPageProps = {
  showHeading?: boolean;
  onLaunched?: (payload: { workspacePath: string }) => void;
  onConfirmed?: () => void;
  onSkipped?: () => void;
};

// ─── Component ─────────────────────────────────────────────────────────────────
export const EmbeddedBootstrapPage: React.FC<EmbeddedBootstrapPageProps> = ({
  showHeading = true,
  onLaunched,
  onConfirmed,
  onSkipped,
}) => {
  const bridge = getDesktopBridge();
  const isDesktop = !!bridge;
  const isWin = typeof navigator !== "undefined" && navigator.userAgent.includes("Windows");

  // Form state
  const [idfPath, setIdfPath]         = useState("");
  const [selectedBoard, setSelectedBoard] = useState(BOARDS[0].id);
  const [customBoard, setCustomBoard] = useState("");
  const [projectDir, setProjectDir]   = useState("");
  const [sandboxPath, setSandboxPath] = useState("");
  const [saving, setSaving]           = useState(false);
  const [error, setError]             = useState<string | null>(null);

  // Pre-fill from saved settings (if any were previously set)
  useEffect(() => {
    if (!bridge?.getSettings) return;
    bridge.getSettings().then((s) => {
      if (typeof s.idf_export_sh === "string" && s.idf_export_sh) setIdfPath(s.idf_export_sh);
      if (typeof s.board_definition === "string" && s.board_definition) {
        const known = BOARDS.find((b) => b.id === s.board_definition);
        if (known) setSelectedBoard(known.id);
        else setCustomBoard(s.board_definition as string);
      }
      if (typeof s.project_dir === "string" && s.project_dir) setProjectDir(s.project_dir);
    }).catch(() => { /* ignore */ });
  }, [bridge]);

  // ── Browse helpers ──────────────────────────────────────────────────────────
  const browseIdf = useCallback(async () => {
    if (!bridge?.browseFile) return;
    const filters = isWin
      ? [{ name: "ESP-IDF Scripts", extensions: ["ps1", "bat", "cmd"] }, { name: "All Files", extensions: ["*"] }]
      : [{ name: "ESP-IDF Scripts", extensions: ["sh"] }, { name: "All Files", extensions: ["*"] }];
    const p = await bridge.browseFile({
      title: isWin ? "Select export.ps1 or export.bat" : "Select export.sh",
      filters,
    });
    if (p) setIdfPath(p);
  }, [bridge, isWin]);

  const browseProjectDir = useCallback(async () => {
    if (!bridge?.browseFolder) return;
    const p = await bridge.browseFolder();
    if (p) setProjectDir(p);
  }, [bridge]);

  const browseSandbox = useCallback(async () => {
    if (!bridge?.browseFolder) return;
    const p = await bridge.browseFolder();
    if (p) setSandboxPath(p);
  }, [bridge]);

  // ── Submit ──────────────────────────────────────────────────────────────────
  const handleConfirm = useCallback(async () => {
    if (!sandboxPath.trim()) {
      setError("Please enter a sandbox folder path for the AI agent.");
      return;
    }
    setError(null);
    setSaving(true);

    try {
      // 1. Persist ESP32 settings to disk via Electron IPC (desktop only)
      if (bridge?.saveWizardSettings) {
        const result = await bridge.saveWizardSettings({
          idf_export_sh:    idfPath.trim() || null,
          board_definition: customBoard.trim() || selectedBoard,
          project_dir:      projectDir.trim() || null,
        });
        if (!result.ok) {
          setError(`Failed to save settings: ${result.error ?? "unknown error"}`);
          setSaving(false);
          return;
        }
      }

      // 2. Tell refact-lsp which folder to use as its sandbox (project_roots)
      onLaunched?.({ workspacePath: sandboxPath.trim() });
      onConfirmed?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSaving(false);
    }
  }, [bridge, idfPath, selectedBoard, customBoard, projectDir, sandboxPath, onLaunched, onConfirmed]);

  const handleSkip = useCallback(async () => {
    // Save whatever is filled in so far, then proceed
    if (bridge?.saveWizardSettings) {
      await bridge.saveWizardSettings({
        idf_export_sh:    idfPath.trim() || null,
        board_definition: customBoard.trim() || selectedBoard,
        project_dir:      projectDir.trim() || null,
      }).catch(() => { /* best-effort */ });
    }
    const sandbox = sandboxPath.trim();
    // IMPORTANT: do not initialize the agent with an empty path, otherwise the
    // code below normalizes it to "/" and we end up giving the agent file:///.
    if (sandbox) {
      onLaunched?.({ workspacePath: sandbox });
    }
    onSkipped?.();
  }, [bridge, idfPath, selectedBoard, customBoard, projectDir, sandboxPath, onLaunched, onSkipped]);

  // ── Board grid styles ───────────────────────────────────────────────────────
  const boardCardStyle = (id: string): React.CSSProperties => ({
    border: `2px solid ${id === selectedBoard && !customBoard ? "var(--accent-9, #4f8ef7)" : "var(--gray-5)"}`,
    borderRadius: 8,
    padding: "10px 14px",
    cursor: "pointer",
    background: id === selectedBoard && !customBoard ? "color-mix(in srgb, var(--accent-9, #4f8ef7) 10%, transparent)" : undefined,
    transition: "border-color 0.15s, background 0.15s",
  });

  return (
    <Card>
      <Flex direction="column" gap="4">
        {showHeading && (
          <Heading as="h3" size="3">
            Configure your ESP32 environment
          </Heading>
        )}

        {/* ── ESP-IDF Path ──────────────────────────────────────── */}
        <Flex direction="column" gap="1">
          <Text size="2" weight="medium">ESP-IDF Export Script</Text>
          <Text size="1" color="gray">
            {isWin
              ? "Path to export.ps1 (usually C:\\Espressif\\frameworks\\esp-idf\\export.ps1)"
              : "Path to export.sh (usually ~/esp/esp-idf/export.sh)"}
          </Text>
          <Flex gap="2">
            <Box flexGrow="1">
              <TextField.Root
                placeholder={isWin ? "C:\\Espressif\\frameworks\\esp-idf\\export.ps1" : "~/esp/esp-idf/export.sh"}
                value={idfPath}
                onChange={(e) => setIdfPath(e.target.value)}
              />
            </Box>
            {isDesktop && (
              <Button variant="outline" onClick={browseIdf} style={{ flexShrink: 0 }}>
                Browse…
              </Button>
            )}
          </Flex>
        </Flex>

        <Separator size="4" />

        {/* ── Board Selection ───────────────────────────────────── */}
        <Flex direction="column" gap="2">
          <Text size="2" weight="medium">Board</Text>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr 1fr",
              gap: 8,
            }}
          >
            {BOARDS.map((board) => (
              <div
                key={board.id}
                style={boardCardStyle(board.id)}
                onClick={() => {
                  setSelectedBoard(board.id);
                  setCustomBoard("");
                }}
              >
                <Text size="2" weight="bold" as="p">{board.name}</Text>
                <Text size="1" color="gray" as="p">{board.desc}</Text>
              </div>
            ))}
          </div>
          <TextField.Root
            placeholder="Custom board ID (leave blank to use selection above)"
            value={customBoard}
            onChange={(e) => setCustomBoard(e.target.value)}
          />
        </Flex>

        <Separator size="4" />

        {/* ── ESP32 Projects Directory ──────────────────────────── */}
        <Flex direction="column" gap="1">
          <Text size="2" weight="medium">ESP32 Projects Directory</Text>
          <Text size="1" color="gray">
            Where your ESP32 firmware projects will be created and stored.
          </Text>
          <Flex gap="2">
            <Box flexGrow="1">
              <TextField.Root
                placeholder={isWin ? "C:\\Users\\you\\craftifai-workspace" : "~/craftifai-workspace"}
                value={projectDir}
                onChange={(e) => setProjectDir(e.target.value)}
              />
            </Box>
            {isDesktop && (
              <Button variant="outline" onClick={browseProjectDir} style={{ flexShrink: 0 }}>
                Browse…
              </Button>
            )}
          </Flex>
        </Flex>

        <Separator size="4" />

        {/* ── AI Agent Sandbox ──────────────────────────────────── */}
        <Flex direction="column" gap="1">
          <Text size="2" weight="medium">AI Agent Sandbox</Text>
          <Text size="1" color="gray">
            The folder the AI agent will index and work within during chat sessions.
          </Text>
          <Flex gap="2">
            <Box flexGrow="1">
              <TextField.Root
                placeholder="/path/to/your/project"
                value={sandboxPath}
                onChange={(e) => setSandboxPath(e.target.value)}
              />
            </Box>
            {isDesktop && (
              <Button variant="outline" onClick={browseSandbox} style={{ flexShrink: 0 }}>
                Browse…
              </Button>
            )}
          </Flex>
        </Flex>

        {error && (
          <Text size="1" color="red">
            {error}
          </Text>
        )}

        {/* ── Actions ───────────────────────────────────────────── */}
        <Flex justify="end" gap="2">
          <Button variant="outline" color="gray" onClick={handleSkip} disabled={saving}>
            Skip
          </Button>
          <Button onClick={handleConfirm} disabled={saving}>
            {saving ? "Saving…" : "Confirm"}
          </Button>
        </Flex>
      </Flex>
    </Card>
  );
};
