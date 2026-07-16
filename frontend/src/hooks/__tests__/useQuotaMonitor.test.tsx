import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useQuotaMonitor, markQuotaExceeded } from '../useQuotaMonitor.js';

describe('useQuotaMonitor', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('reports ~0% usage for empty storage', () => {
    const { result } = renderHook(() => useQuotaMonitor());
    expect(result.current.usagePercent).toBe(0);
    expect(result.current.isNearQuota).toBe(false);
    expect(result.current.quotaExceeded).toBe(false);
  });

  it('does not flag near-quota for small usage', () => {
    localStorage.setItem('k', 'v');
    const { result } = renderHook(() => useQuotaMonitor());
    expect(result.current.isNearQuota).toBe(false);
    expect(result.current.usagePercent).toBeLessThan(80);
  });

  it('flags near-quota once usage exceeds 80% of the 5MB estimate', () => {
    // 5MB limit; usage is chars*2 bytes. >80% ⇒ >2M chars. Use ~2.2M chars.
    localStorage.setItem('big', 'x'.repeat(2_200_000));
    const { result } = renderHook(() => useQuotaMonitor());
    expect(result.current.isNearQuota).toBe(true);
    expect(result.current.usagePercent).toBeGreaterThan(80);
  });

  it('recomputes usage on a window storage event', () => {
    const { result } = renderHook(() => useQuotaMonitor());
    expect(result.current.isNearQuota).toBe(false);

    localStorage.setItem('big', 'y'.repeat(2_200_000));
    act(() => {
      window.dispatchEvent(new StorageEvent('storage'));
    });
    expect(result.current.isNearQuota).toBe(true);
  });
});

describe('markQuotaExceeded', () => {
  it('invokes the setter with true', () => {
    let value = false;
    markQuotaExceeded((v) => {
      value = typeof v === 'function' ? (v as (p: boolean) => boolean)(value) : v;
    });
    expect(value).toBe(true);
  });
});
