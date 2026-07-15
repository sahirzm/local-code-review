import {
  File,
  FileCode,
  FileJson,
  FileText,
  FileCog,
  FileType,
  FileTerminal,
  Database,
  type LucideIcon,
} from 'lucide-react';

// Maps a file extension to a lucide icon. Extension derivation mirrors the
// EXT_TO_LANG precedent in FileDiff.tsx.
const EXT_TO_ICON: Record<string, LucideIcon> = {
  ts: FileCode, tsx: FileCode, js: FileCode, jsx: FileCode, mjs: FileCode, cjs: FileCode,
  py: FileCode, rs: FileCode, go: FileCode, java: FileCode, rb: FileCode, php: FileCode,
  c: FileCode, h: FileCode, cpp: FileCode, hpp: FileCode, cs: FileCode, swift: FileCode, kt: FileCode,
  json: FileJson,
  yaml: FileCog, yml: FileCog, toml: FileCog, ini: FileCog, conf: FileCog, env: FileCog,
  sh: FileTerminal, bash: FileTerminal, zsh: FileTerminal, fish: FileTerminal,
  sql: Database,
  css: FileType, scss: FileType, less: FileType, html: FileType, xml: FileType, svg: FileType,
  md: FileText, mdx: FileText, txt: FileText, rst: FileText,
};

function extensionOf(name: string): string {
  return name.split('.').pop()?.toLowerCase() ?? '';
}

export function getFileIconComponent(name: string): LucideIcon {
  return EXT_TO_ICON[extensionOf(name)] ?? File;
}

export function FileIcon({ name }: { name: string }): React.JSX.Element {
  const Icon = getFileIconComponent(name);
  return <Icon className="tree-file-icon" size={14} aria-hidden="true" />;
}
