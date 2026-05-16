import { useState, useEffect, useCallback } from 'react';

const QUOTA_LIMIT = 5 * 1024 * 1024; // 5MB estimate

interface QuotaStatus {
  usagePercent: number;
  isNearQuota: boolean;
  quotaExceeded: boolean;
}

function estimateUsage(): number {
  let total = 0;
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (key) {
      total += key.length + (localStorage.getItem(key)?.length ?? 0);
    }
  }
  return total * 2; // UTF-16 = 2 bytes per char
}

export function useQuotaMonitor(): QuotaStatus {
  const [status, setStatus] = useState<QuotaStatus>({
    usagePercent: 0,
    isNearQuota: false,
    quotaExceeded: false,
  });

  const check = useCallback(() => {
    const used = estimateUsage();
    const percent = Math.round((used / QUOTA_LIMIT) * 100);
    setStatus({ usagePercent: percent, isNearQuota: percent > 80, quotaExceeded: false });
  }, []);

  useEffect(() => {
    check();
    const handleStorage = () => check();
    window.addEventListener('storage', handleStorage);
    return () => window.removeEventListener('storage', handleStorage);
  }, [check]);

  return status;
}

export function markQuotaExceeded(setter: React.Dispatch<React.SetStateAction<boolean>>): void {
  setter(true);
}
