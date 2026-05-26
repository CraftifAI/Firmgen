/**
 * chatExportHtml.ts
 * Converts a ChatHistoryItem into a self-contained static HTML string suitable
 * for saving to disk.  All CSS is inlined; no external dependencies are needed.
 * Excluded from the export: `system`, `context_file` messages and all internal
 * metadata (token counts, tool IDs, finish reasons, etc.).
 */

import type { ChatHistoryItem } from "../features/History/historySlice";
import type {
  AssistantMessage,
  ChatMessage,
  DiffChunk,
  DiffMessage,
  ToolCall,
  ToolMessage,
  UserMessage,
} from "../services/refact/types";

// ---------------------------------------------------------------------------
// Tiny markdown → HTML renderer (no external deps)
// Handles: fenced code blocks, inline code, bold, italic, links, headings,
// unordered / ordered lists, blockquotes, horizontal rules
// ---------------------------------------------------------------------------

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function renderInlineMarkdown(text: string): string {
  // Bold **text** or __text__
  text = text.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  text = text.replace(/__(.+?)__/g, "<strong>$1</strong>");
  // Italic *text* or _text_
  text = text.replace(/\*([^*\n]+?)\*/g, "<em>$1</em>");
  text = text.replace(/_([^_\n]+?)_/g, "<em>$1</em>");
  // Strikethrough ~~text~~
  text = text.replace(/~~(.+?)~~/g, "<del>$1</del>");
  // Inline code `code`
  text = text.replace(/`([^`\n]+?)`/g, "<code>$1</code>");
  // Links [text](url)
  text = text.replace(
    /\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>',
  );
  return text;
}

function renderMarkdown(raw: string): string {
  const lines = raw.split("\n");
  const outputParts: string[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code blocks ```lang … ```
    const fenceMatch = line.match(/^```(\w*)/);
    if (fenceMatch) {
      const lang = fenceMatch[1] || "";
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      i++; // skip closing ```
      const codeHtml = escapeHtml(codeLines.join("\n"));
      outputParts.push(
        `<pre class="code-block"><code class="lang-${escapeHtml(lang)}">${codeHtml}</code></pre>`,
      );
      continue;
    }

    // Horizontal rule
    if (/^[-*_]{3,}\s*$/.test(line)) {
      outputParts.push("<hr>");
      i++;
      continue;
    }

    // Headings
    const headingMatch = line.match(/^(#{1,6})\s+(.+)/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      outputParts.push(
        `<h${level}>${renderInlineMarkdown(escapeHtml(headingMatch[2]))}</h${level}>`,
      );
      i++;
      continue;
    }

    // Blockquote
    if (line.startsWith("> ")) {
      const quoteLines: string[] = [];
      while (i < lines.length && lines[i].startsWith("> ")) {
        quoteLines.push(lines[i].slice(2));
        i++;
      }
      outputParts.push(
        `<blockquote>${renderMarkdown(quoteLines.join("\n"))}</blockquote>`,
      );
      continue;
    }

    // Unordered list
    if (/^[-*+] /.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*+] /.test(lines[i])) {
        items.push(
          `<li>${renderInlineMarkdown(escapeHtml(lines[i].slice(2)))}</li>`,
        );
        i++;
      }
      outputParts.push(`<ul>${items.join("")}</ul>`);
      continue;
    }

    // Ordered list
    if (/^\d+\. /.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+\. /.test(lines[i])) {
        const text = lines[i].replace(/^\d+\.\s+/, "");
        items.push(
          `<li>${renderInlineMarkdown(escapeHtml(text))}</li>`,
        );
        i++;
      }
      outputParts.push(`<ol>${items.join("")}</ol>`);
      continue;
    }

    // Blank line → paragraph separator
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Regular paragraph
    const paraLines: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].startsWith("#") &&
      !lines[i].startsWith(">") &&
      !/^[-*+] /.test(lines[i]) &&
      !/^\d+\. /.test(lines[i]) &&
      !lines[i].match(/^```/) &&
      !/^[-*_]{3,}\s*$/.test(lines[i])
    ) {
      paraLines.push(lines[i]);
      i++;
    }
    if (paraLines.length > 0) {
      outputParts.push(
        `<p>${renderInlineMarkdown(escapeHtml(paraLines.join(" ")))}</p>`,
      );
    }
  }

  return outputParts.join("\n");
}

// ---------------------------------------------------------------------------
// Message renderers
// ---------------------------------------------------------------------------

function getUserText(msg: UserMessage): string {
  if (typeof msg.content === "string") return msg.content;

  // Multi-modal: collect text parts
  const textParts: string[] = [];
  for (const part of msg.content) {
    if ("type" in part) {
      if (part.type === "text") textParts.push(part.text);
    } else if ("m_type" in part && part.m_type === "text") {
      textParts.push(part.m_content);
    }
  }
  return textParts.join("\n");
}

function hasImages(msg: UserMessage): boolean {
  if (!Array.isArray(msg.content)) return false;
  return msg.content.some((p) => "type" in p && p.type === "image_url");
}

function renderUserMessage(msg: UserMessage): string {
  const text = escapeHtml(getUserText(msg));
  const imgNote = hasImages(msg)
    ? `<div class="img-note">📎 <em>[Image attachment]</em></div>`
    : "";
  return `
    <div class="msg user-msg">
      <div class="msg-avatar user-avatar">U</div>
      <div class="msg-bubble user-bubble">
        <div class="msg-text">${text.replace(/\n/g, "<br>")}</div>
        ${imgNote}
      </div>
    </div>`;
}

function renderToolCalls(toolCalls: ToolCall[]): string {
  return toolCalls
    .map((tc) => {
      const name = tc.function.name ?? "(unknown)";
      let args = "";
      try {
        const parsed = JSON.parse(tc.function.arguments);
        args = escapeHtml(JSON.stringify(parsed, null, 2));
      } catch {
        args = escapeHtml(tc.function.arguments);
      }
      return `
        <details class="tool-call">
          <summary>🔧 <strong>${escapeHtml(name)}</strong></summary>
          <pre class="code-block"><code>${args}</code></pre>
        </details>`;
    })
    .join("\n");
}

function renderAssistantMessage(msg: AssistantMessage): string {
  const parts: string[] = [];

  const text = msg.content;
  if (text) {
    parts.push(`<div class="msg-text">${renderMarkdown(text)}</div>`);
  }

  if (msg.reasoning_content) {
    parts.push(`
      <details class="reasoning">
        <summary>💭 Reasoning</summary>
        <div class="reasoning-body">${escapeHtml(msg.reasoning_content).replace(/\n/g, "<br>")}</div>
      </details>`);
  }

  if (msg.thinking_blocks && msg.thinking_blocks.length > 0) {
    const thinkingHtml = msg.thinking_blocks
      .map((b) => escapeHtml(b.thinking ?? "").replace(/\n/g, "<br>"))
      .join("<hr>");
    parts.push(`
      <details class="reasoning">
        <summary>💭 Thinking</summary>
        <div class="reasoning-body">${thinkingHtml}</div>
      </details>`);
  }

  if (msg.tool_calls && msg.tool_calls.length > 0) {
    parts.push(renderToolCalls(msg.tool_calls));
  }

  if (parts.length === 0) return "";

  return `
    <div class="msg assistant-msg">
      <div class="msg-avatar assistant-avatar">AI</div>
      <div class="msg-bubble assistant-bubble">
        ${parts.join("\n")}
      </div>
    </div>`;
}

function renderToolMessage(msg: ToolMessage): string {
  let content = "";
  if (typeof msg.content.content === "string") {
    content = escapeHtml(msg.content.content);
  } else if (Array.isArray(msg.content.content)) {
    content = msg.content.content
      .map((c) => escapeHtml(c.m_content))
      .join("\n");
  }
  const failed = msg.content.tool_failed;
  const icon = failed ? "❌" : "✅";
  return `
    <div class="msg tool-msg">
      <details class="tool-result ${failed ? "tool-failed" : "tool-ok"}">
        <summary>${icon} Tool result</summary>
        <pre class="code-block"><code>${content}</code></pre>
      </details>
    </div>`;
}

function renderDiffMessage(msg: DiffMessage): string {
  const chunks: DiffChunk[] = Array.isArray(msg.content) ? msg.content : [];
  const diffHtml = chunks
    .map((chunk) => {
      const removed = chunk.lines_remove
        .split("\n")
        .filter(Boolean)
        .map((l) => `<div class="diff-remove">-&nbsp;${escapeHtml(l)}</div>`)
        .join("");
      const added = chunk.lines_add
        .split("\n")
        .filter(Boolean)
        .map((l) => `<div class="diff-add">+&nbsp;${escapeHtml(l)}</div>`)
        .join("");
      const action =
        chunk.file_action === "rename" && chunk.file_name_rename
          ? `${escapeHtml(chunk.file_name)} → ${escapeHtml(chunk.file_name_rename)}`
          : `${escapeHtml(chunk.file_action)}: ${escapeHtml(chunk.file_name)}`;
      return `
        <div class="diff-chunk">
          <div class="diff-header">📄 ${action} (L${chunk.line1}–${chunk.line2})</div>
          <div class="diff-body">${removed}${added}</div>
        </div>`;
    })
    .join("\n");
  return `
    <div class="msg diff-msg">
      <details class="diff-details">
        <summary>📝 Code changes (${chunks.length} chunk${chunks.length !== 1 ? "s" : ""})</summary>
        ${diffHtml}
      </details>
    </div>`;
}

function renderMessage(msg: ChatMessage): string {
  switch (msg.role) {
    case "user":
      return renderUserMessage(msg as UserMessage);
    case "assistant":
      return renderAssistantMessage(msg as AssistantMessage);
    case "tool":
      return renderToolMessage(msg as ToolMessage);
    case "diff":
      return renderDiffMessage(msg as DiffMessage);
    case "plain_text":
    case "cd_instruction":
      return `<div class="msg plain-msg"><div class="msg-bubble plain-bubble">${escapeHtml(msg.content as string).replace(/\n/g, "<br>")}</div></div>`;
    // Intentionally excluded:
    case "system":
    case "context_file":
    default:
      return "";
  }
}

// ---------------------------------------------------------------------------
// Inline CSS (dark CraftifAI theme)
// ---------------------------------------------------------------------------

const INLINE_CSS = `
  /* ── Reset & base ──────────────────────────────────────────────── */
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  html { font-size: 15px; }
  body {
    background: #0d1117;
    color: #e2e8f0;
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    line-height: 1.65;
    min-height: 100vh;
  }
  a { color: #3ec6ff; }

  /* ── Page wrapper ───────────────────────────────────────────────── */
  .page { max-width: 860px; margin: 0 auto; padding: 32px 20px 80px; }

  /* ── Header ─────────────────────────────────────────────────────── */
  .export-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px 24px;
    background: linear-gradient(135deg, #161b22 0%, #0d1117 100%);
    border: 1px solid rgba(62,198,255,.22);
    border-radius: 12px;
    margin-bottom: 32px;
    box-shadow: 0 0 24px rgba(62,198,255,.06);
  }
  .brand-logo {
    width: 40px; height: 40px;
    border-radius: 10px;
    background: linear-gradient(135deg, #3ec6ff 0%, #01fc4c 100%);
    display: flex; align-items: center; justify-content: center;
    font-weight: 800; font-size: 18px; color: #0d1117; flex-shrink: 0;
  }
  .brand-name { font-size: 1.05rem; font-weight: 700; color: #3ec6ff; letter-spacing: .04em; }
  .chat-title { font-size: 1.15rem; font-weight: 600; color: #e2e8f0; margin-top: 2px; }
  .meta { font-size: .78rem; color: #64748b; margin-top: 4px; }
  .meta span { margin-right: 12px; }

  /* ── Messages ───────────────────────────────────────────────────── */
  .messages {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .msg { display: flex; align-items: flex-start; gap: 12px; width: 100%; }

  /* Avatars */
  .msg-avatar {
    flex-shrink: 0;
    width: 34px; height: 34px;
    border-radius: 8px;
    display: flex; align-items: center; justify-content: center;
    font-size: .72rem; font-weight: 700; letter-spacing: .03em;
  }
  .user-avatar { background: rgba(62,198,255,.15); color: #3ec6ff; border: 1px solid rgba(62,198,255,.3); order: 2; }
  .assistant-avatar { background: rgba(1,252,76,.1); color: #01fc4c; border: 1px solid rgba(1,252,76,.25); }

  /* Bubbles */
  .msg-bubble {
    max-width: 78%;
    padding: 12px 16px;
    border-radius: 12px;
    font-size: .92rem;
  }
  .user-msg { flex-direction: row-reverse; }
  .user-bubble {
    background: #1a2233;
    border: 1px solid rgba(62,198,255,.2);
    border-radius: 12px 4px 12px 12px;
  }
  .assistant-bubble {
    background: #141a23;
    border: 1px solid rgba(255,255,255,.07);
    border-radius: 4px 12px 12px 12px;
  }

  /* Tool / diff / plain messages (full-width) */
  .tool-msg, .diff-msg, .plain-msg { flex-direction: column; padding-left: 46px; }
  .plain-bubble {
    background: #0f1925; border: 1px solid rgba(255,255,255,.06);
    border-radius: 8px; padding: 10px 14px; font-size:.88rem; color:#94a3b8;
  }

  /* ── Inline text ────────────────────────────────────────────────── */
  .msg-text p { margin-bottom: .5em; }
  .msg-text p:last-child { margin-bottom: 0; }
  .msg-text h1,.msg-text h2,.msg-text h3,.msg-text h4,.msg-text h5,.msg-text h6 {
    margin: .6em 0 .3em; line-height: 1.3;
    color: #e2e8f0;
  }
  .msg-text ul, .msg-text ol { padding-left: 1.4em; margin-bottom: .5em; }
  .msg-text li { margin-bottom: .2em; }
  .msg-text blockquote {
    border-left: 3px solid rgba(62,198,255,.4);
    padding-left: 12px; margin: .5em 0;
    color: #94a3b8; font-style: italic;
  }
  .msg-text hr { border: none; border-top: 1px solid rgba(255,255,255,.1); margin:.75em 0; }
  .msg-text code {
    background: rgba(255,255,255,.08);
    border-radius: 4px; padding: 1px 5px;
    font-family: 'Cascadia Code','Fira Code',Consolas,monospace;
    font-size: .87em; color: #3ec6ff;
  }
  .msg-text strong { color: #f8fafc; font-weight: 600; }
  .msg-text em { color: #94a3b8; }
  .msg-text a { color: #3ec6ff; }

  /* ── Code blocks ────────────────────────────────────────────────── */
  .code-block {
    background: #0a0e14;
    border: 1px solid rgba(62,198,255,.12);
    border-radius: 8px;
    padding: 14px 16px;
    overflow-x: auto;
    margin: 8px 0 4px;
    font-family: 'Cascadia Code','Fira Code',Consolas,monospace;
    font-size: .84rem;
    line-height: 1.55;
    white-space: pre;
    color: #cdd6f4;
  }
  .code-block code { background: none; padding: 0; border-radius: 0; color: inherit; font-size: inherit; }

  /* ── Tool calls ─────────────────────────────────────────────────── */
  .tool-call, .tool-result, .reasoning, .diff-details {
    background: #10161e;
    border: 1px solid rgba(255,255,255,.07);
    border-radius: 8px;
    padding: 2px 0;
    margin: 4px 0;
    font-size: .88rem;
  }
  .tool-call summary, .tool-result summary, .reasoning summary, .diff-details summary {
    cursor: pointer;
    padding: 9px 14px;
    color: #94a3b8;
    user-select: none;
    list-style: none;
    display: flex; align-items: center; gap: 6px;
  }
  .tool-call summary::-webkit-details-marker,
  .tool-result summary::-webkit-details-marker,
  .reasoning summary::-webkit-details-marker,
  .diff-details summary::-webkit-details-marker { display: none; }
  .tool-call[open] summary,
  .tool-result[open] summary,
  .reasoning[open] summary,
  .diff-details[open] summary { color: #e2e8f0; }
  .tool-ok summary { color: #4ade80; }
  .tool-failed summary { color: #f87171; }
  .reasoning summary { color: #a78bfa; }
  .reasoning-body { padding: 8px 14px 12px; font-size:.86rem; color:#94a3b8; }

  /* ── Diff ───────────────────────────────────────────────────────── */
  .diff-chunk { margin: 8px 14px 12px; }
  .diff-header { font-size:.82rem; color:#64748b; margin-bottom:6px; }
  .diff-body { font-family:'Cascadia Code','Fira Code',Consolas,monospace; font-size:.83rem; }
  .diff-remove { background: rgba(248,113,113,.1); color:#fca5a5; padding:0 6px; }
  .diff-add    { background: rgba(74,222,128,.08); color:#86efac; padding:0 6px; }

  /* ── Image note ─────────────────────────────────────────────────── */
  .img-note { margin-top: 6px; font-size: .82rem; color: #64748b; }

  /* ── Footer ─────────────────────────────────────────────────────── */
  .export-footer {
    margin-top: 48px;
    text-align: center;
    font-size: .76rem;
    color: #334155;
  }
`;

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

export function chatToHtml(item: ChatHistoryItem): string {
  const title = escapeHtml(item.title || "Untitled chat");
  const model = escapeHtml(item.model || "");
  const exportDate = new Date().toLocaleString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
  const createdAt = item.createdAt
    ? new Date(item.createdAt).toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
      })
    : "";

  const messagesHtml = item.messages
    .map(renderMessage)
    .filter(Boolean)
    .join("\n");

  const messageCount = item.messages.filter(
    (m) => m.role === "user" || m.role === "assistant",
  ).length;

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${title} — CraftifAI Export</title>
  <style>${INLINE_CSS}</style>
</head>
<body>
  <div class="page">

    <div class="export-header">
      <div class="brand-logo">C</div>
      <div>
        <div class="brand-name">CraftifAI</div>
        <div class="chat-title">${title}</div>
        <div class="meta">
          ${model ? `<span>🤖 ${model}</span>` : ""}
          ${createdAt ? `<span>📅 Started ${createdAt}</span>` : ""}
          <span>💬 ${messageCount} message${messageCount !== 1 ? "s" : ""}</span>
          <span>📤 Exported ${exportDate}</span>
        </div>
      </div>
    </div>

    <div class="messages">
      ${messagesHtml}
    </div>

    <div class="export-footer">
      Exported from CraftifAI &mdash; ${exportDate}
    </div>

  </div>
</body>
</html>`;
}
