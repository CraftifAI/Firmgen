import React, { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import classNames from "classnames";
import mermaid from "mermaid";

import styles from "./MermaidDiagram.module.css";

type MermaidDiagramProps = {
  code?: string;
  className?: string;
  loading?: boolean;
  generationError?: string | null;
  onRegenerate?: () => void;
  /** Unused in canvas chrome; kept for API compatibility. */
  title?: string;
};

const clamp = (value: number, min: number, max: number) =>
  Math.max(min, Math.min(max, value));

function rgbComponentsToHex(r: number, g: number, b: number): string {
  const hex = (n: number) =>
    Math.max(0, Math.min(255, Math.round(n)))
      .toString(16)
      .padStart(2, "0");
  return `#${hex(r)}${hex(g)}${hex(b)}`;
}

/** Convert any CSS color string (including display-p3) to #rrggbb via canvas. */
function cssColorToHex(cssColor: string, fallback: string): string {
  const trimmed = cssColor.trim();
  if (!trimmed) return fallback;
  if (/^#[0-9a-f]{3,8}$/i.test(trimmed)) {
    return trimmed.length === 4
      ? `#${trimmed[1]}${trimmed[1]}${trimmed[2]}${trimmed[2]}${trimmed[3]}${trimmed[3]}`
      : trimmed.slice(0, 7);
  }
  const rgbMatch = trimmed.match(
    /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i,
  );
  if (rgbMatch) {
    return rgbComponentsToHex(
      Number(rgbMatch[1]),
      Number(rgbMatch[2]),
      Number(rgbMatch[3]),
    );
  }

  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  const ctx = canvas.getContext("2d");
  if (!ctx) return fallback;
  try {
    ctx.fillStyle = trimmed;
    ctx.fillRect(0, 0, 1, 1);
    const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
    return rgbComponentsToHex(r, g, b);
  } catch {
    return fallback;
  }
}

/** Mermaid only accepts hex/rgb — resolve CSS vars from the active theme scope. */
function cssVarForMermaid(
  name: string,
  fallback: string,
  scope?: HTMLElement | null,
): string {
  const scopedValue = scope
    ? getComputedStyle(scope).getPropertyValue(name).trim()
    : "";
  if (scopedValue) {
    return cssColorToHex(scopedValue, fallback);
  }
  const rootValue = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return cssColorToHex(rootValue, fallback);
}

function applyMermaidTheme(scope?: HTMLElement | null): void {
  const color = (name: string, fallback: string) =>
    cssVarForMermaid(name, fallback, scope);

  const canvasBg = color("--gray-1", "#111113");
  const mainBkg = color("--gray-2", "#1a1a1d");
  const actorBkg = color("--gray-3", "#232326");
  const noteBkg = color("--gray-3", "#232326");
  const labelBkg = color("--gray-3", "#232326");
  const lldText = color("--gray-12", "#eceef0");

  const themeVariables = {
    darkMode: "true",
    fontSize: "16px",
    actorFontSize: "16px",
    noteFontSize: "15px",
    messageFontSize: "15px",
    background: canvasBg,
    mainBkg,
    primaryColor: actorBkg,
    primaryTextColor: lldText,
    primaryBorderColor: color("--gray-6", "#46464b"),
    secondaryColor: actorBkg,
    tertiaryColor: color("--gray-5", "#3a3a3f"),
    lineColor: color("--gray-8", "#5f5f65"),
    textColor: lldText,
    actorBkg,
    actorBorder: color("--gray-6", "#46464b"),
    actorTextColor: lldText,
    actorLineColor: color("--gray-8", "#5f5f65"),
    signalColor: color("--gray-8", "#5f5f65"),
    signalTextColor: lldText,
    labelBoxBkgColor: labelBkg,
    labelBoxBorderColor: color("--gray-6", "#46464b"),
    labelTextColor: lldText,
    loopTextColor: lldText,
    noteBkgColor: noteBkg,
    noteTextColor: lldText,
    noteBorderColor: color("--gray-6", "#46464b"),
    activationBkgColor: color("--gray-4", "#2e2e32"),
    activationBorderColor: color("--gray-8", "#5f5f65"),
    sequenceNumberColor: color("--gray-11", "#b0b4ba"),
  };

  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: "base",
    themeVariables,
  });
}

function normalizeMermaidCode(rawCode: string): string {
  const normalized = rawCode.replace(/\r\n?/g, "\n").trim();
  if (!normalized) return "";
  if (normalized.startsWith("```")) {
    const lines = normalized.split("\n");
    if (lines.length >= 2 && lines[0].startsWith("```")) {
      const closingIdx = lines.lastIndexOf("```");
      if (closingIdx > 0) {
        return lines.slice(1, closingIdx).join("\n").trim();
      }
      return lines.slice(1).join("\n").trim();
    }
  }
  return normalized;
}

export const MermaidDiagram: React.FC<MermaidDiagramProps> = ({
  code,
  className,
  loading = false,
  generationError = null,
  onRegenerate,
}) => {
  const reactId = useId();
  const renderId = useMemo(
    () => `pir-mermaid-${reactId.replace(/[:]/g, "-")}-${Math.random().toString(16).slice(2)}`,
    [reactId],
  );
  const [svg, setSvg] = useState<string>("");
  const [renderError, setRenderError] = useState<string | null>(null);
  const [rendering, setRendering] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isPanning, setIsPanning] = useState(false);

  const canvasRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{ pointerId: number; x: number; y: number } | null>(null);
  const lastGoodSvgRef = useRef<string>("");

  const resetView = useCallback(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, []);

  const onWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    e.preventDefault();
    const delta = e.deltaY < 0 ? 0.1 : -0.1;
    setZoom((prev) => clamp(prev + delta, 0.5, 3));
  }, []);

  const onPointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    dragRef.current = { pointerId: e.pointerId, x: e.clientX, y: e.clientY };
    setIsPanning(true);
    e.currentTarget.setPointerCapture(e.pointerId);
  }, []);

  const onPointerMove = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (!dragRef.current || dragRef.current.pointerId !== e.pointerId) return;
    const dx = e.clientX - dragRef.current.x;
    const dy = e.clientY - dragRef.current.y;
    dragRef.current = { pointerId: e.pointerId, x: e.clientX, y: e.clientY };
    setPan((prev) => ({ x: prev.x + dx, y: prev.y + dy }));
  }, []);

  const endPan = useCallback((e?: React.PointerEvent<HTMLDivElement>) => {
    if (e && dragRef.current && dragRef.current.pointerId === e.pointerId) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
    dragRef.current = null;
    setIsPanning(false);
  }, []);

  const toggleFullscreen = useCallback(async () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    if (document.fullscreenElement === canvas) {
      await document.exitFullscreen();
      return;
    }
    await canvas.requestFullscreen();
  }, []);

  const exportSvg = useCallback(() => {
    if (!svg.trim()) return;
    const blob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "pir-sequence-diagram.svg";
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }, [svg]);

  useEffect(() => {
    const trimmed = normalizeMermaidCode(code ?? "");
    if (!trimmed) {
      setSvg("");
      lastGoodSvgRef.current = "";
      setRenderError(null);
      return;
    }

    let canceled = false;
    setRenderError(null);
    setRendering(true);

    try {
      applyMermaidTheme(canvasRef.current);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setRenderError(msg);
      setRendering(false);
      return;
    }

    void mermaid
      .render(renderId, trimmed)
      .then((res) => {
        if (canceled) return;
        setSvg(res.svg);
        lastGoodSvgRef.current = res.svg;
        setRendering(false);
      })
      .catch((e: unknown) => {
        if (canceled) return;
        const msg = e instanceof Error ? e.message : String(e);
        setRenderError(msg);
        setSvg((prev) => prev || lastGoodSvgRef.current);
        setRendering(false);
      });

    return () => {
      canceled = true;
      setRendering(false);
    };
  }, [code, renderId]);

  const combinedError = renderError ?? generationError;
  const showEmpty = !combinedError && !(code ?? "").trim();
  const showDiagram = !combinedError && Boolean((code ?? "").trim());

  return (
    <div ref={canvasRef} className={classNames(styles.canvas, className)}>
      {loading || rendering ? (
        <div className={styles.status}>Rendering sequence diagram…</div>
      ) : null}

      {combinedError ? (
        <div className={styles.errorPanel}>
          <div className={styles.errorTitle}>Mermaid render failed</div>
          <pre className={styles.errorBody}>{combinedError}</pre>
          {onRegenerate ? (
            <button type="button" onClick={onRegenerate}>
              Regenerate Sequence Diagram
            </button>
          ) : null}
        </div>
      ) : null}

      {showEmpty ? (
        <div className={styles.emptyState}>
          <span>No sequence diagram was generated yet.</span>
          {onRegenerate ? (
            <button type="button" onClick={onRegenerate}>
              Generate now
            </button>
          ) : null}
        </div>
      ) : null}

      {showDiagram ? (
        <div
          className={classNames(styles.viewport, isPanning && styles.viewportPanning)}
          onWheel={onWheel}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={endPan}
          onPointerCancel={endPan}
        >
          <div
            className={styles.stage}
            style={{
              transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            }}
            dangerouslySetInnerHTML={{ __html: svg }}
          />
          <div className={styles.controls} aria-label="Sequence diagram controls">
            <button type="button" title="Zoom in" onClick={() => setZoom((z) => clamp(z + 0.1, 0.5, 3))}>
              +
            </button>
            <button type="button" title="Zoom out" onClick={() => setZoom((z) => clamp(z - 0.1, 0.5, 3))}>
              −
            </button>
            <button type="button" title="Reset view" onClick={resetView}>
              ⊙
            </button>
            <button type="button" title="Export SVG" onClick={exportSvg} disabled={!svg.trim()}>
              ↓
            </button>
            <button type="button" title="Fullscreen" onClick={() => void toggleFullscreen()}>
              ⛶
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
};
