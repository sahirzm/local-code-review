import { describe, it, expect } from 'vitest';
import { generateClientMarkdown } from '../client-markdown.js';
import type { Comment } from '../../shared/types.js';

const now = '2024-01-01T00:00:00.000Z';

function comment(overrides: Partial<Comment> = {}): Comment {
  return {
    id: '1',
    type: 'line',
    category: 'fix',
    text: 'Fix this',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

describe('generateClientMarkdown', () => {
  it('returns placeholder for empty comments', () => {
    const md = generateClientMarkdown([]);
    expect(md).toContain('No comments.');
    expect(md).toContain('# Code Review Comments');
  });

  it('renders overall comments under Overall section', () => {
    const md = generateClientMarkdown([comment({ type: 'overall', text: 'Looks good overall' })]);
    expect(md).toContain('## Overall');
    expect(md).toContain('[fix] Looks good overall');
  });

  it('groups file comments by file path', () => {
    const md = generateClientMarkdown([
      comment({ type: 'file', filePath: 'src/a.ts', text: 'File level note' }),
      comment({ type: 'file', filePath: 'src/b.ts', text: 'Another file' }),
    ]);
    expect(md).toContain('## src/a.ts');
    expect(md).toContain('## src/b.ts');
    expect(md).toContain('[fix] File level note');
    expect(md).toContain('[fix] Another file');
  });

  it('renders line comments with line label', () => {
    const md = generateClientMarkdown([
      comment({ type: 'line', filePath: 'foo.ts', startLine: 42, text: 'Bug here' }),
    ]);
    expect(md).toContain('### Line 42');
    expect(md).toContain('[fix] Bug here');
  });

  it('renders range comments with line range label', () => {
    const md = generateClientMarkdown([
      comment({ type: 'range', filePath: 'foo.ts', startLine: 10, endLine: 20, text: 'Refactor this block' }),
    ]);
    expect(md).toContain('### Lines 10-20');
    expect(md).toContain('[fix] Refactor this block');
  });

  it('sorts line comments by start line within a file', () => {
    const md = generateClientMarkdown([
      comment({ id: '2', type: 'line', filePath: 'a.ts', startLine: 50, text: 'Second' }),
      comment({ id: '1', type: 'line', filePath: 'a.ts', startLine: 10, text: 'First' }),
    ]);
    const firstIdx = md.indexOf('First');
    const secondIdx = md.indexOf('Second');
    expect(firstIdx).toBeLessThan(secondIdx);
  });

  it('shows category in brackets', () => {
    const md = generateClientMarkdown([
      comment({ type: 'overall', category: 'nit', text: 'Minor style' }),
      comment({ id: '2', type: 'overall', category: 'question', text: 'Why?' }),
      comment({ id: '3', type: 'overall', category: 'suggestion', text: 'Maybe try X' }),
    ]);
    expect(md).toContain('[nit] Minor style');
    expect(md).toContain('[question] Why?');
    expect(md).toContain('[suggestion] Maybe try X');
  });

  it('does not produce triple newlines', () => {
    const md = generateClientMarkdown([
      comment({ type: 'overall', text: 'A' }),
      comment({ id: '2', type: 'line', filePath: 'b.ts', startLine: 1, text: 'B' }),
    ]);
    expect(md).not.toContain('\n\n\n');
  });

  it('ends with a single newline', () => {
    const md = generateClientMarkdown([comment({ type: 'overall', text: 'Test' })]);
    expect(md).toMatch(/[^\n]\n$/);
  });

  it('handles mixed overall, file, and line comments', () => {
    const md = generateClientMarkdown([
      comment({ id: '1', type: 'overall', text: 'Overall note' }),
      comment({ id: '2', type: 'file', filePath: 'x.ts', text: 'File note' }),
      comment({ id: '3', type: 'line', filePath: 'x.ts', startLine: 5, text: 'Line note' }),
    ]);
    expect(md).toContain('## Overall');
    expect(md).toContain('## x.ts');
    expect(md).toContain('### Line 5');
  });
});
