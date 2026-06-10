import hljs from "highlight.js/lib/core";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import python from "highlight.js/lib/languages/python";
import javascript from "highlight.js/lib/languages/javascript";
import typescript from "highlight.js/lib/languages/typescript";
import json from "highlight.js/lib/languages/json";
import rust from "highlight.js/lib/languages/rust";
import bash from "highlight.js/lib/languages/bash";
import yaml from "highlight.js/lib/languages/yaml";
import xml from "highlight.js/lib/languages/xml";
import css from "highlight.js/lib/languages/css";
import markdown from "highlight.js/lib/languages/markdown";
import cmake from "highlight.js/lib/languages/cmake";
import plaintext from "highlight.js/lib/languages/plaintext";

const REGISTERED = new Set<string>();

function registerLanguage(name: string, module: unknown) {
  if (REGISTERED.has(name)) return;
  hljs.registerLanguage(name, module as Parameters<typeof hljs.registerLanguage>[1]);
  REGISTERED.add(name);
}

function ensureLanguage(language: string) {
  switch (language) {
    case "c":
      registerLanguage("c", c);
      break;
    case "cpp":
      registerLanguage("cpp", cpp);
      break;
    case "python":
      registerLanguage("python", python);
      break;
    case "javascript":
    case "jsx":
      registerLanguage("javascript", javascript);
      break;
    case "typescript":
    case "tsx":
      registerLanguage("typescript", typescript);
      break;
    case "json":
      registerLanguage("json", json);
      break;
    case "rust":
      registerLanguage("rust", rust);
      break;
    case "bash":
      registerLanguage("bash", bash);
      break;
    case "yaml":
      registerLanguage("yaml", yaml);
      break;
    case "html":
    case "xml":
      registerLanguage("xml", xml);
      break;
    case "css":
      registerLanguage("css", css);
      break;
    case "markdown":
      registerLanguage("markdown", markdown);
      break;
    case "cmake":
    case "dockerfile":
      registerLanguage("cmake", cmake);
      break;
    default:
      registerLanguage("plaintext", plaintext);
      break;
  }
}

export function highlightLine(text: string, language: string): string {
  if (!text) return "";
  const lang = language === "plaintext" ? "plaintext" : language;
  ensureLanguage(lang);
  try {
    if (hljs.getLanguage(lang)) {
      return hljs.highlight(text, { language: lang, ignoreIllegals: true }).value;
    }
  } catch {
    // fall through
  }
  return escapeHtml(text);
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
