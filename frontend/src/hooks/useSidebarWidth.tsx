import { useCallback, useEffect, useRef, useState } from 'react';

export const SIDEBAR_WIDTH_STORAGE_KEY = 'local-review:sidebar-width';
export const SIDEBAR_MIN_WIDTH = 220;
export const SIDEBAR_MAX_WIDTH = 640;
export const SIDEBAR_DEFAULT_WIDTH = 260;

export function clampWidth(px: number): number {
  if (!Number.isFinite(px)) return SIDEBAR_DEFAULT_WIDTH;
  return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, Math.round(px)));
}

function readStored(): number {
  try {
    const raw = localStorage.getItem(SIDEBAR_WIDTH_STORAGE_KEY);
    if (raw === null) return SIDEBAR_DEFAULT_WIDTH;
    return clampWidth(Number.parseInt(raw, 10));
  } catch {
    return SIDEBAR_DEFAULT_WIDTH;
  }
}

interface SidebarWidth {
  width: number;
  resizing: boolean;
  startResize: (e: React.MouseEvent) => void;
}

export function useSidebarWidth(): SidebarWidth {
  const [width, setWidth] = useState<number>(() => readStored());
  const [resizing, setResizing] = useState(false);
  const widthRef = useRef(width);
  widthRef.current = width;

  const startResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setResizing(true);

    const onMove = (ev: MouseEvent) => {
      setWidth(clampWidth(ev.clientX));
    };
    const onUp = () => {
      setResizing(false);
      try {
        localStorage.setItem(SIDEBAR_WIDTH_STORAGE_KEY, String(widthRef.current));
      } catch {
        // ignore persistence failures
      }
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };

    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  }, []);

  useEffect(() => {
    if (!resizing) return;
    document.body.style.userSelect = 'none';
    document.body.style.cursor = 'col-resize';
    return () => {
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
    };
  }, [resizing]);

  return { width, resizing, startResize };
}
