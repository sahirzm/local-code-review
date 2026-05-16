import type { ReviewSession } from '../../../shared/types.js';

const SESSION_PREFIX = 'local-review:';
const EXPIRY_MS = 14 * 24 * 60 * 60 * 1000; // 14 days

export function hashRepoPath(repoPath: string): string {
  let hash = 0;
  for (let i = 0; i < repoPath.length; i++) {
    hash = ((hash << 5) - hash + repoPath.charCodeAt(i)) | 0;
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

export function getSessionKey(repoPathHash: string, commitRange: string): string {
  return `${SESSION_PREFIX}${repoPathHash}:${commitRange}`;
}

function migrateSession(data: Record<string, unknown>): ReviewSession {
  return {
    version: 2,
    commitRange: (data.commitRange as string) ?? '',
    repoPath: (data.repoPath as string) ?? '',
    repoPathHash: (data.repoPathHash as string) ?? '',
    comments: (data.comments as ReviewSession['comments']) ?? [],
    reviewedFiles: (data.reviewedFiles as string[]) ?? [],
    viewMode: (data.viewMode as ReviewSession['viewMode']) ?? 'split',
    createdAt: (data.createdAt as string) ?? new Date().toISOString(),
    lastAccessedAt: (data.lastAccessedAt as string) ?? new Date().toISOString(),
  };
}

export function saveSession(key: string, session: ReviewSession): void {
  try {
    localStorage.setItem(key, JSON.stringify(session));
  } catch (e: unknown) {
    if (e instanceof DOMException && e.name === 'QuotaExceededError') {
      console.warn('localStorage quota exceeded, cleaning expired sessions');
      cleanExpiredSessions();
      try {
        localStorage.setItem(key, JSON.stringify(session));
      } catch {
        console.error('localStorage quota exceeded even after cleanup');
      }
    }
  }
}

export function loadSession(key: string): ReviewSession | null {
  const raw = localStorage.getItem(key);
  if (!raw) return null;
  try {
    const data = JSON.parse(raw) as Record<string, unknown>;
    const session = data.version === 2 ? (data as unknown as ReviewSession) : migrateSession(data);
    const lastAccessed = new Date(session.lastAccessedAt).getTime();
    if (Date.now() - lastAccessed > EXPIRY_MS) {
      localStorage.removeItem(key);
      return null;
    }
    return session;
  } catch {
    localStorage.removeItem(key);
    return null;
  }
}

export function clearSession(key: string): void {
  localStorage.removeItem(key);
}

export function cleanExpiredSessions(): void {
  const now = Date.now();
  for (let i = localStorage.length - 1; i >= 0; i--) {
    const key = localStorage.key(i);
    if (!key?.startsWith(SESSION_PREFIX) || key === `${SESSION_PREFIX.slice(0, -1)}:preferences`) continue;
    try {
      const data = JSON.parse(localStorage.getItem(key)!) as { lastAccessedAt?: string };
      if (!data.lastAccessedAt || now - new Date(data.lastAccessedAt).getTime() > EXPIRY_MS) {
        localStorage.removeItem(key);
      }
    } catch {
      localStorage.removeItem(key);
    }
  }
}
