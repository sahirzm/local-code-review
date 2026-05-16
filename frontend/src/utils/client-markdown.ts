import type { Comment } from '../../../shared/types.js';

function formatComment(c: Comment): string {
  return `- [${c.category}] ${c.text}`;
}

/** Simplified client-side markdown generator (no code context available). */
export function generateClientMarkdown(comments: Comment[]): string {
  if (comments.length === 0) return '# Code Review Comments\n\nNo comments.\n';

  const parts: string[] = ['# Code Review Comments\n'];

  const overall = comments.filter((c) => c.type === 'overall');
  const byFile = new Map<string, Comment[]>();

  for (const c of comments) {
    if (c.type === 'overall') continue;
    const key = c.filePath ?? '';
    if (!byFile.has(key)) byFile.set(key, []);
    byFile.get(key)!.push(c);
  }

  if (overall.length > 0) {
    parts.push('## Overall\n');
    for (const c of overall) parts.push(formatComment(c));
    parts.push('');
  }

  for (const [filePath, fileComments] of byFile) {
    parts.push(`## ${filePath}\n`);

    const fileLevel = fileComments.filter((c) => c.type === 'file');
    const lineComments = fileComments
      .filter((c) => c.type === 'line' || c.type === 'range')
      .sort((a, b) => (a.startLine ?? 0) - (b.startLine ?? 0));

    if (fileLevel.length > 0) {
      for (const c of fileLevel) parts.push(formatComment(c));
      parts.push('');
    }

    for (const c of lineComments) {
      const start = c.startLine ?? 0;
      const end = c.endLine ?? start;
      const lineLabel = end > start ? `Lines ${start}-${end}` : `Line ${start}`;
      parts.push(`### ${lineLabel}\n`);
      parts.push(formatComment(c));
      parts.push('');
    }
  }

  return parts.join('\n').replace(/\n{3,}/g, '\n\n').trimEnd() + '\n';
}

export function downloadMarkdown(markdown: string, filename = 'review-comments.md'): void {
  const blob = new Blob([markdown], { type: 'text/markdown' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
