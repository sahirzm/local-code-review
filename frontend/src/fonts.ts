import type { CodeFontId, UiFontId } from './shared/types.js';

export interface FontDef<Id> {
  id: Id;
  label: string;
  /** CSS font-family stack; the leading family degrades to the trailing generic. */
  stack: string;
}

// The default entries reproduce the previously hard-coded --font-mono / --font-sans
// stacks, so preferences that predate this control render identically. No font
// files are bundled; each stack degrades to whatever the user has installed.
export const CODE_FONTS: FontDef<CodeFontId>[] = [
  { id: 'system-mono', label: 'System Default', stack: "'SF Mono', 'JetBrains Mono', 'Fira Code', Menlo, Consolas, monospace" },
  { id: 'jetbrains-mono', label: 'JetBrains Mono', stack: "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace" },
  { id: 'fira-code', label: 'Fira Code', stack: "'Fira Code', ui-monospace, Menlo, Consolas, monospace" },
  { id: 'sf-mono', label: 'SF Mono', stack: "'SF Mono', ui-monospace, Menlo, Consolas, monospace" },
  { id: 'source-code-pro', label: 'Source Code Pro', stack: "'Source Code Pro', ui-monospace, Menlo, Consolas, monospace" },
  { id: 'ibm-plex-mono', label: 'IBM Plex Mono', stack: "'IBM Plex Mono', ui-monospace, Menlo, Consolas, monospace" },
  { id: 'menlo-consolas', label: 'Menlo / Consolas', stack: "Menlo, Consolas, 'Courier New', monospace" },
];

export const UI_FONTS: FontDef<UiFontId>[] = [
  { id: 'system-sans', label: 'System Default', stack: "'Inter var', 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" },
  { id: 'inter', label: 'Inter', stack: "'Inter var', 'Inter', -apple-system, BlinkMacSystemFont, sans-serif" },
  { id: 'roboto', label: 'Roboto', stack: "Roboto, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif" },
  { id: 'segoe-ui', label: 'Segoe UI', stack: "'Segoe UI', -apple-system, BlinkMacSystemFont, Roboto, sans-serif" },
  { id: 'helvetica', label: 'Helvetica', stack: "'Helvetica Neue', Helvetica, Arial, sans-serif" },
];

export const DEFAULT_CODE_FONT: CodeFontId = 'system-mono';
export const DEFAULT_UI_FONT: UiFontId = 'system-sans';

const CODE_FONT_IDS = new Set<string>(CODE_FONTS.map((f) => f.id));
const UI_FONT_IDS = new Set<string>(UI_FONTS.map((f) => f.id));

export function normalizeCodeFontId(value: unknown): CodeFontId {
  return typeof value === 'string' && CODE_FONT_IDS.has(value) ? (value as CodeFontId) : DEFAULT_CODE_FONT;
}

export function normalizeUiFontId(value: unknown): UiFontId {
  return typeof value === 'string' && UI_FONT_IDS.has(value) ? (value as UiFontId) : DEFAULT_UI_FONT;
}

export function resolveCodeFontStack(id: CodeFontId): string {
  return (CODE_FONTS.find((f) => f.id === id) ?? CODE_FONTS[0]).stack;
}

export function resolveUiFontStack(id: UiFontId): string {
  return (UI_FONTS.find((f) => f.id === id) ?? UI_FONTS[0]).stack;
}
