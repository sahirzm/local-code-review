import type {
  ReviewMetadata,
  DiffResponse,
  FileChange,
  ParsedFileDiff,
  FinishResponse,
} from '../shared/types.js';

const CSRF = 'test-csrf-token';

const FILE_CHANGES: FileChange[] = [
  { path: 'src/index.ts', status: 'modified', additions: 3, deletions: 1 },
  { path: 'src/util/helpers.py', status: 'added', additions: 10, deletions: 0 },
  { path: 'README.md', status: 'modified', additions: 2, deletions: 2 },
  { path: 'package.json', status: 'modified', additions: 1, deletions: 0 },
];

function metadata(): ReviewMetadata {
  return {
    repoName: 'demo-repo',
    commitRange: 'main..feature',
    baseRef: 'main',
    headRef: 'feature',
    files: FILE_CHANGES,
    timestamp: '2026-01-01T00:00:00.000Z',
    csrfToken: CSRF,
  };
}

function diffFile(path: string, status: FileChange['status']): ParsedFileDiff {
  return {
    oldPath: path,
    newPath: path,
    status,
    additions: 2,
    deletions: 1,
    isBinary: false,
    isLarge: false,
    hunks: [
      {
        oldStart: 1,
        oldLines: 2,
        newStart: 1,
        newLines: 3,
        content: '@@ -1,2 +1,3 @@',
        changes: [
          { type: 'normal', content: ' context', oldLineNumber: 1, newLineNumber: 1 },
          { type: 'insert', content: '+added line', newLineNumber: 2 },
          { type: 'delete', content: '-removed line', oldLineNumber: 2 },
        ],
      },
    ],
  };
}

function diff(): DiffResponse {
  return {
    files: FILE_CHANGES.map((f) => diffFile(f.path, f.status)),
  };
}

function finish(): FinishResponse {
  return { success: true, outputPath: '/tmp/review-demo.md', markdown: '# Review\n\nno comments' };
}

function jsonResponse(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  });
}

export interface MockApiHandle {
  csrfToken: string;
  restore: () => void;
}

/**
 * Replaces `window.fetch` with a stub that answers the app's `/api/v1/*` calls
 * from in-memory fixtures, so browser tests need no backend. Call `restore()`
 * in test teardown.
 */
export function mockApi(): MockApiHandle {
  const original = window.fetch;

  window.fetch = ((input: RequestInfo | URL): Promise<Response> => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    if (url.includes('/api/v1/metadata')) return Promise.resolve(jsonResponse(metadata()));
    if (url.includes('/api/v1/diff')) return Promise.resolve(jsonResponse(diff()));
    if (url.includes('/api/v1/finish')) return Promise.resolve(jsonResponse(finish()));
    if (url.includes('/api/v1/shutdown')) return Promise.resolve(jsonResponse({ success: true }));
    return Promise.resolve(new Response('not found', { status: 404 }));
  }) as typeof window.fetch;

  return { csrfToken: CSRF, restore: () => { window.fetch = original; } };
}
