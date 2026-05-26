import React, { useState, useCallback, useEffect, useRef } from "react";
import {
  Text,
  TextField,
  Button,
  Box,
  Select,
  Dialog,
  Tabs,
} from "@radix-ui/themes";
import styles from "./EmbeddedBootstrapPage.module.css";

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
  sandbox_path: string | null;
}

interface BoardInfo {
  board_id: string;
  name: string;
  variant: string;
  description?: string;
  chip: string;
  flash_size: string;
  psram_size: string;
  is_builtin?: boolean;
}

// Fallback list shown while the API is loading or unreachable
const BOARDS_FALLBACK: BoardInfo[] = [
  { board_id: "esp32-s3-DevKitM-1-N16R8",            name: "ESP32-S3 DevKitM-1",  variant: "N16R8",   chip: "esp32s3", flash_size: "16MB", psram_size: "8MB",  is_builtin: true },
  { board_id: "esp32-s3-devkitc-1-n32r8v",           name: "ESP32-S3 DevKitC-1",  variant: "N32R8V",  chip: "esp32s3", flash_size: "32MB", psram_size: "8MB",  is_builtin: true },
  { board_id: "esp32-wroom-32",                       name: "ESP32-WROOM-32",       variant: "Classic", chip: "esp32",   flash_size: "4MB",  psram_size: "",     is_builtin: true },
  { board_id: "ESP32-S3-WROOM-1-N16R8-touch-lcd-4b", name: "ESP32-S3 Touch LCD",   variant: "N16R8",   chip: "esp32s3", flash_size: "16MB", psram_size: "",     is_builtin: true },
];

const API_BASE = "http://127.0.0.1:8002";

const CHIP_OPTIONS = ["esp32", "esp32s3", "esp32s2", "esp32c3", "esp32c6", "esp32h2"];

function boardLabel(b: BoardInfo): string {
  const parts = [b.name];
  if (b.variant) parts.push(`(${b.variant})`);
  const specs: string[] = [];
  if (b.flash_size) specs.push(b.flash_size + " flash");
  if (b.psram_size) specs.push(b.psram_size + " PSRAM");
  if (specs.length) parts.push("·", specs.join(" · "));
  return parts.join(" ");
}

// ─── Empty build form ──────────────────────────────────────────────────────────
function emptyBuildForm() {
  return {
    board_id: "",
    name: "",
    variant: "",
    chip: "esp32s3",
    description: "",
    flash_size: "16MB",
    psram_enabled: false,
    psram_size: "8MB",
    uart_tx: "43",
    uart_rx: "44",
    usb_d_minus: "19",
    usb_d_plus: "20",
    led_pin: "",
    led_driver: "gpio",
    button_pin: "0",
    safe_pins: "",
    restricted_pins: "",
    restricted_reasons: "",
  };
}

type BuildForm = ReturnType<typeof emptyBuildForm>;

function buildFormToJson(f: BuildForm): Record<string, unknown> {
  const parsePins = (s: string) =>
    s.split(",").map((x) => x.trim()).filter(Boolean).map(Number).filter((n) => !isNaN(n));

  const restrictedReasons: Record<string, string> = {};
  f.restricted_reasons.split("\n").forEach((line) => {
    const idx = line.indexOf(":");
    if (idx > 0) {
      restrictedReasons[line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
    }
  });

  return {
    schema_version: "1.1",
    board_id: f.board_id.trim(),
    name: f.name.trim(),
    variant: f.variant.trim(),
    description: f.description.trim(),
    chip: { type: f.chip },
    hardware: {
      flash: { size: f.flash_size },
      psram: f.psram_enabled ? { enabled: true, size: f.psram_size } : { enabled: false },
      uart_console: { tx: Number(f.uart_tx), rx: Number(f.uart_rx) },
      usb_jtag: { supported: true, d_minus: Number(f.usb_d_minus), d_plus: Number(f.usb_d_plus) },
    },
    gpio: {
      ...(f.led_pin ? { led: { pin: Number(f.led_pin), driver: f.led_driver } } : {}),
      button: { pin: Number(f.button_pin), pull: "pullup" },
      safe_pins: parsePins(f.safe_pins),
      restricted_pins: parsePins(f.restricted_pins),
      restricted_reasons: restrictedReasons,
    },
  };
}

// ─── Props ─────────────────────────────────────────────────────────────────────
type EmbeddedBootstrapPageProps = {
  showHeading?: boolean;
  onLaunched?: (payload: { workspacePath: string }) => boolean | void | Promise<boolean | void>;
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

  // ── Main form state ──────────────────────────────────────────────────────────
  const [idfPath, setIdfPath] = useState("");
  const [selectedBoard, setSelectedBoard] = useState<string>(BOARDS_FALLBACK[0].board_id);
  const [customBoard, setCustomBoard] = useState("");
  const [projectDir, setProjectDir] = useState("");
  const [sandboxPath, setSandboxPath] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [availableBoards, setAvailableBoards] = useState<BoardInfo[]>(BOARDS_FALLBACK);

  // ── Add-board modal state ────────────────────────────────────────────────────
  const [modalOpen, setModalOpen] = useState(false);
  const [modalTab, setModalTab] = useState<"import" | "build" | "pdf">("import");
  const [modalBusy, setModalBusy] = useState(false);
  const [modalError, setModalError] = useState<string | null>(null);

  // Tab 1 — Import JSON
  const [importJson, setImportJson] = useState("");
  const jsonFileRef = useRef<HTMLInputElement>(null);

  // Tab 2 — Build manually
  const [buildForm, setBuildForm] = useState<BuildForm>(emptyBuildForm());
  const [buildStep, setBuildStep] = useState(0);

  // Tab 3 — Generate from PDF
  const [pdfFile, setPdfFile] = useState<File | null>(null);
  const [pdfHints, setPdfHints] = useState("");
  const [pdfDraft, setPdfDraft] = useState<Record<string, unknown> | null>(null);
  const pdfFileRef = useRef<HTMLInputElement>(null);

  // ── Fetch dynamic board list ─────────────────────────────────────────────────
  const refreshBoards = useCallback(() => {
    fetch(`${API_BASE}/v1/boards`)
      .then((r) => r.json())
      .then((data: { boards: BoardInfo[] }) => {
        if (data.boards && data.boards.length > 0) setAvailableBoards(data.boards);
      })
      .catch(() => {});
  }, []);

  useEffect(() => { refreshBoards(); }, [refreshBoards]);

  // ── Pre-fill from saved settings ─────────────────────────────────────────────
  useEffect(() => {
    if (!bridge?.getSettings) return;
    bridge.getSettings().then((s) => {
      if (typeof s.idf_export_sh === "string" && s.idf_export_sh) setIdfPath(s.idf_export_sh);
      if (typeof s.board_definition === "string" && s.board_definition) {
        setSelectedBoard(s.board_definition);
        setCustomBoard("");
      }
      if (typeof s.project_dir === "string" && s.project_dir) setProjectDir(s.project_dir);
      if (typeof s.sandbox_path === "string" && s.sandbox_path) setSandboxPath(s.sandbox_path);
    }).catch(() => {});
  }, [bridge]);

  // ── Browse helpers ────────────────────────────────────────────────────────────
  const browseIdf = useCallback(async () => {
    if (!bridge?.browseFile) return;
    const filters = isWin
      ? [{ name: "ESP-IDF Scripts", extensions: ["ps1", "bat", "cmd"] }, { name: "All Files", extensions: ["*"] }]
      : [{ name: "ESP-IDF Scripts", extensions: ["sh"] }, { name: "All Files", extensions: ["*"] }];
    const p = await bridge.browseFile({ title: isWin ? "Select export.ps1 or export.bat" : "Select export.sh", filters });
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

  // ── Save a board JSON via API and refresh dropdown ────────────────────────────
  const saveBoard = useCallback(async (boardData: Record<string, unknown>): Promise<boolean> => {
    const boardId = String(boardData.board_id || "").trim();
    if (!boardId) { setModalError("board_id is required."); return false; }

    const res = await fetch(`${API_BASE}/v1/boards`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(boardData),
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ detail: res.statusText }));
      setModalError(String(err.detail || "Failed to save board."));
      return false;
    }
    refreshBoards();
    setSelectedBoard(boardId);
    setCustomBoard("");
    return true;
  }, [refreshBoards]);

  // ── Delete a user board ───────────────────────────────────────────────────────
  const deleteBoard = useCallback(async (boardId: string) => {
    if (!confirm(`Delete board "${boardId}"? This cannot be undone.`)) return;
    const res = await fetch(`${API_BASE}/v1/boards/${encodeURIComponent(boardId)}`, { method: "DELETE" });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ detail: res.statusText }));
      setError(String(err.detail || "Delete failed."));
      return;
    }
    refreshBoards();
    if (selectedBoard === boardId) setSelectedBoard(BOARDS_FALLBACK[0].board_id);
  }, [refreshBoards, selectedBoard]);

  // ── Modal: open helpers ───────────────────────────────────────────────────────
  const openModal = useCallback((tab: typeof modalTab = "import") => {
    setModalTab(tab);
    setModalError(null);
    setImportJson("");
    setBuildForm(emptyBuildForm());
    setBuildStep(0);
    setPdfFile(null);
    setPdfHints("");
    setPdfDraft(null);
    setModalOpen(true);
  }, []);

  // ── Modal: open board in editor ───────────────────────────────────────────────
  const editBoard = useCallback(async (boardId: string) => {
    try {
      const res = await fetch(`${API_BASE}/v1/boards/${encodeURIComponent(boardId)}`);
      if (!res.ok) throw new Error("Failed to load board.");
      const data = await res.json() as Record<string, unknown>;
      const hw = (data.hardware as Record<string, unknown>) || {};
      const gpio = (data.gpio as Record<string, unknown>) || {};
      const flash = (hw.flash as Record<string, unknown>) || {};
      const psram = (hw.psram as Record<string, unknown>) || {};
      const uart = (hw.uart_console as Record<string, unknown>) || {};
      const usb = (hw.usb_jtag as Record<string, unknown>) || {};
      const led = (gpio.led as Record<string, unknown>) || {};
      const button = (gpio.button as Record<string, unknown>) || {};
      const reasons = (gpio.restricted_reasons as Record<string, string>) || {};

      setBuildForm({
        board_id: String(data.board_id || ""),
        name: String(data.name || ""),
        variant: String(data.variant || ""),
        chip: String((data.chip as Record<string,unknown>)?.type || "esp32s3"),
        description: String(data.description || ""),
        flash_size: String(flash.size || "16MB"),
        psram_enabled: Boolean(psram.enabled),
        psram_size: String(psram.size || "8MB"),
        uart_tx: String(uart.tx ?? "43"),
        uart_rx: String(uart.rx ?? "44"),
        usb_d_minus: String(usb.d_minus ?? "19"),
        usb_d_plus: String(usb.d_plus ?? "20"),
        led_pin: String(led.pin ?? ""),
        led_driver: String(led.driver || "gpio"),
        button_pin: String(button.pin ?? "0"),
        safe_pins: ((gpio.safe_pins as number[]) || []).join(", "),
        restricted_pins: ((gpio.restricted_pins as number[]) || []).join(", "),
        restricted_reasons: Object.entries(reasons).map(([k, v]) => `${k}: ${v}`).join("\n"),
      });
      setBuildStep(0);
      setModalTab("build");
      setModalError(null);
      setModalOpen(true);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  // ── Tab 1: Import JSON ────────────────────────────────────────────────────────
  const handleJsonFileChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => setImportJson(String(ev.target?.result || ""));
    reader.readAsText(file);
  }, []);

  const handleImportSave = useCallback(async () => {
    setModalError(null);
    setModalBusy(true);
    try {
      const parsed = JSON.parse(importJson);
      const ok = await saveBoard(parsed);
      if (ok) setModalOpen(false);
    } catch {
      setModalError("Invalid JSON — please check the file content.");
    } finally {
      setModalBusy(false);
    }
  }, [importJson, saveBoard]);

  // ── Tab 2: Build manually ─────────────────────────────────────────────────────
  const bf = (key: keyof BuildForm) => (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
  ) => setBuildForm((f) => ({ ...f, [key]: e.target.value }));

  const handleBuildSave = useCallback(async () => {
    setModalError(null);
    if (!buildForm.board_id.trim()) { setModalError("Board ID is required."); return; }
    if (!buildForm.name.trim()) { setModalError("Board name is required."); return; }
    setModalBusy(true);
    try {
      const data = buildFormToJson(buildForm);
      const ok = await saveBoard(data);
      if (ok) setModalOpen(false);
    } finally {
      setModalBusy(false);
    }
  }, [buildForm, saveBoard]);

  // ── Tab 3: Generate from PDF ──────────────────────────────────────────────────
  const handlePdfFileChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setPdfFile(e.target.files?.[0] ?? null);
    setPdfDraft(null);
  }, []);

  const handlePdfExtract = useCallback(async () => {
    if (!pdfFile) { setModalError("Please select a PDF file."); return; }
    setModalError(null);
    setModalBusy(true);
    setPdfDraft(null);
    try {
      const form = new FormData();
      form.append("file", pdfFile);
      if (pdfHints) form.append("hints", pdfHints);
      const res = await fetch(`${API_BASE}/v1/boards/generate-from-pdf`, { method: "POST", body: form });
      if (!res.ok) {
        const err = await res.json().catch(() => ({ detail: res.statusText }));
        setModalError(String(err.detail || "Extraction failed."));
        return;
      }
      const { board } = await res.json() as { board: Record<string, unknown> };
      setPdfDraft(board);
      // Pre-fill the build form with the draft so the user can review/edit
      const hw = (board.hardware as Record<string, unknown>) || {};
      const gpio = (board.gpio as Record<string, unknown>) || {};
      const flash = (hw.flash as Record<string, unknown>) || {};
      const psram = (hw.psram as Record<string, unknown>) || {};
      const uart = (hw.uart_console as Record<string, unknown>) || {};
      const usb = (hw.usb_jtag as Record<string, unknown>) || {};
      const led = (gpio.led as Record<string, unknown>) || {};
      const button = (gpio.button as Record<string, unknown>) || {};
      const reasons = (gpio.restricted_reasons as Record<string, string>) || {};
      setBuildForm({
        board_id: String(board.board_id || ""),
        name: String(board.name || ""),
        variant: String(board.variant || ""),
        chip: String((board.chip as Record<string,unknown>)?.type || "esp32s3"),
        description: String(board.description || ""),
        flash_size: String(flash.size || "16MB"),
        psram_enabled: Boolean(psram.enabled),
        psram_size: String(psram.size || "8MB"),
        uart_tx: String(uart.tx ?? ""),
        uart_rx: String(uart.rx ?? ""),
        usb_d_minus: String(usb.d_minus ?? ""),
        usb_d_plus: String(usb.d_plus ?? ""),
        led_pin: String(led.pin ?? ""),
        led_driver: String(led.driver || "gpio"),
        button_pin: String(button.pin ?? ""),
        safe_pins: ((gpio.safe_pins as number[]) || []).join(", "),
        restricted_pins: ((gpio.restricted_pins as number[]) || []).join(", "),
        restricted_reasons: Object.entries(reasons).map(([k, v]) => `${k}: ${v}`).join("\n"),
      });
      setBuildStep(0);
      setModalTab("build");
    } finally {
      setModalBusy(false);
    }
  }, [pdfFile, pdfHints]);

  // ── Main form submit ──────────────────────────────────────────────────────────
  const handleConfirm = useCallback(async () => {
    if (!sandboxPath.trim()) { setError("Please enter a sandbox folder path for the AI agent."); return; }
    setError(null);
    setSaving(true);
    try {
      if (bridge?.saveWizardSettings) {
        const result = await bridge.saveWizardSettings({
          idf_export_sh: idfPath.trim() || null,
          board_definition: customBoard.trim() || selectedBoard,
          project_dir: projectDir.trim() || null,
          sandbox_path: sandboxPath.trim() || null,
        });
        if (!result.ok) { setError(`Failed to save settings: ${result.error ?? "unknown error"}`); return; }
      }
      const launched = onLaunched?.({ workspacePath: sandboxPath.trim() });
      const ok = launched === undefined ? true : await Promise.resolve(launched);
      if (!ok) { setError("Could not reach the AI agent after it restarted. Wait a few seconds and try Confirm again, or restart the app."); return; }
      onConfirmed?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [bridge, idfPath, selectedBoard, customBoard, projectDir, sandboxPath, onLaunched, onConfirmed]);

  const handleSkip = useCallback(async () => {
    setError(null);
    setSaving(true);
    try {
      if (bridge?.saveWizardSettings) {
        await bridge.saveWizardSettings({
          idf_export_sh: idfPath.trim() || null,
          board_definition: customBoard.trim() || selectedBoard,
          project_dir: projectDir.trim() || null,
          sandbox_path: sandboxPath.trim() || null,
        }).catch(() => {});
      }
      const sandbox = sandboxPath.trim();
      if (sandbox) {
        const launched = onLaunched?.({ workspacePath: sandbox });
        const ok = launched === undefined ? true : await Promise.resolve(launched);
        if (!ok) { setError("Could not reach the AI agent. Try again in a moment or leave sandbox empty and Skip."); return; }
      }
      onSkipped?.();
    } finally {
      setSaving(false);
    }
  }, [bridge, idfPath, selectedBoard, customBoard, projectDir, sandboxPath, onLaunched, onSkipped]);

  const browseBtn = (run: () => Promise<void> | void) => (
    <Button type="button" variant="solid" color="gray" className={styles.browseBtn} onClick={() => { void run(); }}>
      Browse...
    </Button>
  );

  // ── User boards (for manage row) ──────────────────────────────────────────────
  const userBoards = availableBoards.filter((b) => !b.is_builtin);

  // ── Build form steps ──────────────────────────────────────────────────────────
  const buildSteps = [
    // Step 0: Identity
    <div key="identity" className={styles.buildStep}>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Board ID <span className={styles.required}>*</span></label>
        <input className={styles.buildInput} value={buildForm.board_id} onChange={bf("board_id")}
          placeholder="my-esp32-board" />
        <span className={styles.buildHint}>Lowercase, hyphens only</span>
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Name <span className={styles.required}>*</span></label>
        <input className={styles.buildInput} value={buildForm.name} onChange={bf("name")} placeholder="My ESP32 Board" />
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Variant</label>
        <input className={styles.buildInput} value={buildForm.variant} onChange={bf("variant")} placeholder="N16R8" />
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Chip</label>
        <select className={styles.buildSelect} value={buildForm.chip}
          onChange={(e) => setBuildForm((f) => ({ ...f, chip: e.target.value }))}>
          {CHIP_OPTIONS.map((c) => <option key={c} value={c}>{c}</option>)}
        </select>
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Description</label>
        <input className={styles.buildInput} value={buildForm.description} onChange={bf("description")}
          placeholder="Optional one-line description" />
      </div>
    </div>,

    // Step 1: Hardware
    <div key="hardware" className={styles.buildStep}>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Flash size</label>
        <select className={styles.buildSelect} value={buildForm.flash_size}
          onChange={(e) => setBuildForm((f) => ({ ...f, flash_size: e.target.value }))}>
          {["2MB","4MB","8MB","16MB","32MB"].map((s) => <option key={s} value={s}>{s}</option>)}
        </select>
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>PSRAM</label>
        <label className={styles.checkLabel}>
          <input type="checkbox" checked={buildForm.psram_enabled}
            onChange={(e) => setBuildForm((f) => ({ ...f, psram_enabled: e.target.checked }))} />
          &nbsp;Enabled
        </label>
        {buildForm.psram_enabled && (
          <select className={styles.buildSelect} value={buildForm.psram_size}
            onChange={(e) => setBuildForm((f) => ({ ...f, psram_size: e.target.value }))}>
            {["2MB","4MB","8MB","16MB"].map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
        )}
      </div>
      <div className={styles.buildRow2}>
        <div>
          <label className={styles.buildLabel}>UART TX pin</label>
          <input className={styles.buildInputSm} value={buildForm.uart_tx} onChange={bf("uart_tx")} placeholder="43" />
        </div>
        <div>
          <label className={styles.buildLabel}>UART RX pin</label>
          <input className={styles.buildInputSm} value={buildForm.uart_rx} onChange={bf("uart_rx")} placeholder="44" />
        </div>
        <div>
          <label className={styles.buildLabel}>USB D- pin</label>
          <input className={styles.buildInputSm} value={buildForm.usb_d_minus} onChange={bf("usb_d_minus")} placeholder="19" />
        </div>
        <div>
          <label className={styles.buildLabel}>USB D+ pin</label>
          <input className={styles.buildInputSm} value={buildForm.usb_d_plus} onChange={bf("usb_d_plus")} placeholder="20" />
        </div>
      </div>
    </div>,

    // Step 2: GPIO
    <div key="gpio" className={styles.buildStep}>
      <div className={styles.buildRow2}>
        <div>
          <label className={styles.buildLabel}>LED pin</label>
          <input className={styles.buildInputSm} value={buildForm.led_pin} onChange={bf("led_pin")} placeholder="38" />
        </div>
        <div>
          <label className={styles.buildLabel}>LED driver</label>
          <select className={styles.buildSelectSm} value={buildForm.led_driver}
            onChange={(e) => setBuildForm((f) => ({ ...f, led_driver: e.target.value }))}>
            <option value="gpio">gpio</option>
            <option value="ws2812">ws2812 (RGB)</option>
            <option value="apa102">apa102</option>
          </select>
        </div>
        <div>
          <label className={styles.buildLabel}>Button pin</label>
          <input className={styles.buildInputSm} value={buildForm.button_pin} onChange={bf("button_pin")} placeholder="0" />
        </div>
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Safe GPIO pins</label>
        <input className={styles.buildInput} value={buildForm.safe_pins} onChange={bf("safe_pins")}
          placeholder="1, 2, 4, 5, 6, 7, ..." />
        <span className={styles.buildHint}>Comma-separated pin numbers</span>
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Restricted GPIO pins</label>
        <input className={styles.buildInput} value={buildForm.restricted_pins} onChange={bf("restricted_pins")}
          placeholder="0, 19, 20, 26, 27, ..." />
      </div>
      <div className={styles.buildRow}>
        <label className={styles.buildLabel}>Restricted reasons</label>
        <textarea className={styles.buildTextarea} value={buildForm.restricted_reasons}
          onChange={bf("restricted_reasons")}
          placeholder={"0: Boot button\n19: USB D-\n20: USB D+"} rows={4} />
        <span className={styles.buildHint}>One per line: pin: reason</span>
      </div>
    </div>,
  ];

  const buildStepLabels = ["Identity", "Hardware", "GPIO"];

  // ── Modal content ─────────────────────────────────────────────────────────────
  const modalContent = (
    <Tabs.Root
      value={modalTab}
      onValueChange={(v) => { setModalTab(v as typeof modalTab); setModalError(null); }}
      className={styles.modalTabsRoot}
    >
      <Tabs.List className={styles.modalTabs}>
        <Tabs.Trigger value="import">Import JSON</Tabs.Trigger>
        <Tabs.Trigger value="build">Build Manually</Tabs.Trigger>
        <Tabs.Trigger value="pdf">Generate from PDF</Tabs.Trigger>
      </Tabs.List>

      {/* ── Tab 1: Import JSON ── */}
      <Tabs.Content value="import" className={styles.tabContent}>
        <Text as="p" className={styles.tabDesc}>
          Select a board definition <code>.json</code> file, or paste JSON directly.
        </Text>
        <div className={styles.importActions}>
          <input ref={jsonFileRef} type="file" accept=".json" style={{ display: "none" }}
            onChange={handleJsonFileChange} />
          <button className={styles.filePickBtn} type="button"
            onClick={() => jsonFileRef.current?.click()}>
            📂 Choose JSON file
          </button>
        </div>
        <textarea
          className={styles.jsonTextarea}
          value={importJson}
          onChange={(e) => setImportJson(e.target.value)}
          placeholder='{"board_id": "my-board", "name": "My Board", ...}'
          rows={8}
          spellCheck={false}
        />
      </Tabs.Content>

      {/* ── Tab 2: Build Manually ── */}
      <Tabs.Content value="build" className={styles.tabContent}>
        {pdfDraft && (
          <div className={styles.draftBanner}>
            ✨ Pre-filled from PDF — review and adjust before saving.
          </div>
        )}
        <div className={styles.stepNav}>
          {buildStepLabels.map((label, i) => (
            <button key={label} type="button"
              className={`${styles.stepPill} ${i === buildStep ? styles.stepPillActive : ""} ${i < buildStep ? styles.stepPillDone : ""}`}
              onClick={() => setBuildStep(i)}>
              {i < buildStep ? "✓ " : ""}{label}
            </button>
          ))}
        </div>
        {buildSteps[buildStep]}
      </Tabs.Content>

      {/* ── Tab 3: Generate from PDF ── */}
      <Tabs.Content value="pdf" className={styles.tabContent}>
        <Text as="p" className={styles.tabDesc}>
          Upload an ESP32 board datasheet or schematic PDF. The AI will extract a draft board
          definition which you can review in the <strong>Build Manually</strong> tab before saving.
        </Text>
        <div className={styles.importActions}>
          <input ref={pdfFileRef} type="file" accept=".pdf" style={{ display: "none" }}
            onChange={handlePdfFileChange} />
          <button className={styles.filePickBtn} type="button"
            onClick={() => pdfFileRef.current?.click()}>
            📄 {pdfFile ? pdfFile.name : "Choose PDF file"}
          </button>
        </div>
        <div className={styles.buildRow}>
          <label className={styles.buildLabel}>Hints (optional)</label>
          <input className={styles.buildInput} value={pdfHints}
            onChange={(e) => setPdfHints(e.target.value)}
            placeholder="e.g. ESP32-S3, 16MB flash, custom pinout" />
          <span className={styles.buildHint}>Extra context to help the AI</span>
        </div>
      </Tabs.Content>

      {/* ── Shared footer — always visible outside the scrollable tab area ── */}
      {modalError && <Text className={styles.modalError}>{modalError}</Text>}
      <div className={styles.modalFooter}>
        <Dialog.Close>
          <button className={styles.cancelBtn} type="button">Cancel</button>
        </Dialog.Close>

        {/* Action button changes per tab */}
        {modalTab === "import" && (
          <button className={styles.saveBtn} type="button"
            disabled={!importJson.trim() || modalBusy}
            onClick={() => { void handleImportSave(); }}>
            {modalBusy ? "Saving…" : "Save Board"}
          </button>
        )}
        {modalTab === "build" && (
          <div style={{ display: "flex", gap: "0.5rem" }}>
            {buildStep > 0 && (
              <button className={styles.cancelBtn} type="button" onClick={() => setBuildStep((s) => s - 1)}>
                ← Back
              </button>
            )}
            {buildStep < buildStepLabels.length - 1 ? (
              <button className={styles.saveBtn} type="button" onClick={() => setBuildStep((s) => s + 1)}>
                Next →
              </button>
            ) : (
              <button className={styles.saveBtn} type="button"
                disabled={!buildForm.board_id.trim() || !buildForm.name.trim() || modalBusy}
                onClick={() => { void handleBuildSave(); }}>
                {modalBusy ? "Saving…" : "Save Board"}
              </button>
            )}
          </div>
        )}
        {modalTab === "pdf" && (
          <button className={styles.saveBtn} type="button"
            disabled={!pdfFile || modalBusy}
            onClick={() => { void handlePdfExtract(); }}>
            {modalBusy ? "Extracting…" : "Extract & Review →"}
          </button>
        )}
      </div>
    </Tabs.Root>
  );

  // ── Main form ─────────────────────────────────────────────────────────────────
  const formInner = (
    <div className={styles.formStack}>
      {/* ESP-IDF Export Script */}
      <div className={styles.fieldBlock}>
        <Text as="label" className={styles.label}>Export Script</Text>
        <TextField.Root
          placeholder={isWin ? "C:\\Espressif\\frameworks\\esp-idf\\export.ps1" : "~/esp/esp-idf/export.sh"}
          value={idfPath} onChange={(e) => setIdfPath(e.target.value)} size="3" className={styles.fieldRoot}>
          {isDesktop && (
            <TextField.Slot side="right" className={styles.browseSlot}>{browseBtn(browseIdf)}</TextField.Slot>
          )}
        </TextField.Root>
      </div>

      {/* Board */}
      <div className={styles.boardField}>
        <div className={styles.boardLabelRow}>
          <Text as="label" className={styles.label}>Board</Text>
          <Dialog.Root open={modalOpen} onOpenChange={setModalOpen}>
            <Dialog.Trigger>
              <button className={styles.addBoardBtn} type="button" onClick={() => openModal("import")}>
                + Add Board
              </button>
            </Dialog.Trigger>
            <Dialog.Content className={styles.modalContent} aria-describedby={undefined}>
              <Dialog.Title className={styles.modalTitle}>Add New Board</Dialog.Title>
              {modalContent}
            </Dialog.Content>
          </Dialog.Root>
        </div>

        <Select.Root
          value={customBoard ? "__custom__" : selectedBoard}
          onValueChange={(val) => { if (val === "__custom__") return; setSelectedBoard(val); setCustomBoard(""); }}
          size="3">
          <Select.Trigger className={styles.boardSelectTrigger} />
          <Select.Content>
            {availableBoards.map((board) => (
              <Select.Item key={board.board_id} value={board.board_id}>
                {boardLabel(board)}{!board.is_builtin ? " ✎" : ""}
              </Select.Item>
            ))}
            {customBoard && (
              <Select.Item value="__custom__">Custom: {customBoard}</Select.Item>
            )}
          </Select.Content>
        </Select.Root>

        {/* User boards management row */}
        {userBoards.length > 0 && (
          <div className={styles.userBoardsList}>
            <Text as="p" className={styles.userBoardsLabel}>My Boards:</Text>
            {userBoards.map((b) => (
              <div key={b.board_id} className={styles.userBoardRow}>
                <span className={styles.userBoardName}>{b.name}{b.variant ? ` (${b.variant})` : ""}</span>
                <button className={styles.iconBtn} type="button" title="Edit" onClick={() => { void editBoard(b.board_id); }}>✎</button>
                <button className={styles.iconBtnDanger} type="button" title="Delete" onClick={() => { void deleteBoard(b.board_id); }}>✕</button>
              </div>
            ))}
          </div>
        )}

        <TextField.Root
          placeholder="Custom board ID (leave blank to use selection above)"
          value={customBoard}
          onChange={(e) => { setCustomBoard(e.target.value); if (e.target.value) setSelectedBoard(""); }}
          size="3" className={styles.fieldRoot} />
      </div>

      {/* ESP32 Projects Directory */}
      <div className={styles.fieldBlock}>
        <Text as="label" className={styles.label}>Projects Directory</Text>
        <TextField.Root
          placeholder={isWin ? "/path/to/your/project" : "~/craftifai-workspace"}
          value={projectDir} onChange={(e) => setProjectDir(e.target.value)} size="3" className={styles.fieldRoot}>
          {isDesktop && (
            <TextField.Slot side="right" className={styles.browseSlot}>{browseBtn(browseProjectDir)}</TextField.Slot>
          )}
        </TextField.Root>
      </div>

      {/* AI Agent Sandbox */}
      <div className={styles.fieldBlock}>
        <Text as="label" className={styles.label}>AI Agent Sandbox</Text>
        <TextField.Root
          placeholder="C:\\path\\to\\your\\craftifai-workspace"
          value={sandboxPath} onChange={(e) => setSandboxPath(e.target.value)} size="3" className={styles.fieldRoot}>
          {isDesktop && (
            <TextField.Slot side="right" className={styles.browseSlot}>{browseBtn(browseSandbox)}</TextField.Slot>
          )}
        </TextField.Root>
      </div>

      {error && <Text size="2" className={styles.errorText}>{error}</Text>}

      <div className={styles.actions}>
        <Button type="button" variant="outline" size="3" className={styles.skipBtn}
          onClick={() => { void handleSkip(); }} disabled={saving}>
          Skip
        </Button>
        <Button type="button" size="3" variant="solid" className={styles.confirmBtn}
          onClick={() => { void handleConfirm(); }} disabled={saving}>
          {saving ? "Preparing workspace…" : "Confirm"}
        </Button>
      </div>
    </div>
  );

  if (!showHeading) {
    return <Box className={styles.embeddedShell}>{formInner}</Box>;
  }

  return (
    <Box className={styles.page}>
      {/* <Flex align="center" justify="between" gap="3" wrap="wrap" className={styles.header}>
        <Flex align="center" gap="3" wrap="wrap" style={{ minWidth: 0 }}>
          <div className={styles.logoMark} aria-hidden>F</div>
          <Flex align="center" gap="3" wrap="wrap" style={{ minWidth: 0 }}>
            <Text weight="bold" size="4" style={{ color: "#fff" }}>FirmGen</Text>
            <Box style={{ width: 1, height: 22, background: "var(--fg-border)", flexShrink: 0 }} />
            <Text size="2" style={{ color: "var(--fg-muted)" }}>Firmware Setup and Configuration Screen</Text>
          </Flex>
        </Flex>
        <button type="button" aria-label="Profile" className={styles.profileBtn}>
          <UserCircle size={28} strokeWidth={1.25} />
        </button>
      </Flex> */}
      <Box className={styles.main}>
        <Box className={styles.inner}>{formInner}</Box>
      </Box>
    </Box>
  );
};
