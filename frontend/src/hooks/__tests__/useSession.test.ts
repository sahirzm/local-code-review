import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  hashRepoPath,
  getSessionKey,
  saveSession,
  loadSession,
  clearSession,
  cleanExpiredSessions,
} from '../useSession.js';
import type { ReviewSession } from '../../shared/types.js';

function session(overrides: Partial<ReviewSession> = {}): ReviewSession {
  return {
    version: 2,
    commitRange: 'abc..def',
    repoPath: '/repo',
    repoPathHash: '12345678',
    comments: [],
    reviewedFiles: [],
    viewMode: 'split',
    createdAt: new Date().toISOString(),
    lastAccessedAt: new Date().toISOString(),
    ...overrides,
  };
}

describe('hashRepoPath', () => {
  it('returns 8-char hex string', () => {
    const hash = hashRepoPath('/some/repo/path');
    expect(hash).toMatch(/^[0-9a-f]{8}$/);
  });

  it('returns consistent results', () => {
    expect(hashRepoPath('test')).toBe(hashRepoPath('test'));
  });

  it('returns different hashes for different inputs', () => {
    expect(hashRepoPath('a')).not.toBe(hashRepoPath('b'));
  });

  it('handles empty string', () => {
    const hash = hashRepoPath('');
    expect(hash).toMatch(/^[0-9a-f]{8}$/);
    expect(hash).toBe('00000000');
  });
});

describe('getSessionKey', () => {
  it('builds key with prefix', () => {
    expect(getSessionKey('abcd1234', 'abc..def')).toBe('local-review:abcd1234:abc..def');
  });
});

describe('saveSession / loadSession', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('saves and loads a session', () => {
    const s = session();
    const key = 'local-review:test:abc..def';
    saveSession(key, s);
    const loaded = loadSession(key);
    expect(loaded).toEqual(s);
  });

  it('returns null for non-existent key', () => {
    expect(loadSession('local-review:nope:x')).toBeNull();
  });

  it('returns null and removes expired sessions', () => {
    const expired = session({
      lastAccessedAt: new Date(Date.now() - 15 * 24 * 60 * 60 * 1000).toISOString(),
    });
    const key = 'local-review:test:old';
    localStorage.setItem(key, JSON.stringify(expired));
    expect(loadSession(key)).toBeNull();
    expect(localStorage.getItem(key)).toBeNull();
  });

  it('returns null and removes unparseable data', () => {
    const key = 'local-review:test:bad';
    localStorage.setItem(key, '{invalid json');
    expect(loadSession(key)).toBeNull();
    expect(localStorage.getItem(key)).toBeNull();
  });

  it('migrates v1 sessions (no version field)', () => {
    const key = 'local-review:test:v1';
    const v1Data = {
      commitRange: 'a..b',
      repoPath: '/repo',
      repoPathHash: '00000000',
      comments: [],
      reviewedFiles: [],
      lastAccessedAt: new Date().toISOString(),
    };
    localStorage.setItem(key, JSON.stringify(v1Data));
    const loaded = loadSession(key);
    expect(loaded).not.toBeNull();
    expect(loaded!.version).toBe(2);
    expect(loaded!.viewMode).toBe('split');
  });
});

describe('clearSession', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('removes the session from localStorage', () => {
    const key = 'local-review:test:clear';
    localStorage.setItem(key, JSON.stringify(session()));
    clearSession(key);
    expect(localStorage.getItem(key)).toBeNull();
  });
});

describe('cleanExpiredSessions', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('removes expired sessions', () => {
    const expiredKey = 'local-review:x:expired';
    const freshKey = 'local-review:x:fresh';

    localStorage.setItem(
      expiredKey,
      JSON.stringify({ lastAccessedAt: new Date(Date.now() - 15 * 24 * 60 * 60 * 1000).toISOString() }),
    );
    localStorage.setItem(
      freshKey,
      JSON.stringify({ lastAccessedAt: new Date().toISOString() }),
    );

    cleanExpiredSessions();
    expect(localStorage.getItem(expiredKey)).toBeNull();
    expect(localStorage.getItem(freshKey)).not.toBeNull();
  });

  it('removes entries with unparseable data', () => {
    const badKey = 'local-review:x:corrupt';
    localStorage.setItem(badKey, 'not json');
    cleanExpiredSessions();
    expect(localStorage.getItem(badKey)).toBeNull();
  });

  it('does not remove non-session keys', () => {
    localStorage.setItem('other-key', 'value');
    cleanExpiredSessions();
    expect(localStorage.getItem('other-key')).toBe('value');
  });

  it('does not remove preferences key', () => {
    const prefKey = 'local-review:preferences';
    localStorage.setItem(prefKey, JSON.stringify({ theme: 'dark' }));
    cleanExpiredSessions();
    expect(localStorage.getItem(prefKey)).not.toBeNull();
  });
});
