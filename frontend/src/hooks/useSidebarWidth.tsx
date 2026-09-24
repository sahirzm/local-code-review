import { useCallback, useEffect, useRef, useState } from 'react';
import {
  SIDEBAR_DEFAULT_WIDTH,
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_MIN_WIDTH,
  clampSidebarWidth,
  loadPreferences,
  patchPreferences,
} from '../utils/preferences.js';

export { SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_DEFAULT_WIDTH };
/** @deprecated use `clampSidebarWidth` from utils/preferences. */
export const clampWidth = clampSidebarWidth;

interface SidebarWidth {
  width: number;
  resizing: boolean;
  startResize: (e: React.MouseEvent) => void;
}

export function useSidebarWidth(): SidebarWidth {
  // Lives in the consolidated preferences blob. Its own `local-review:*` key
  // was indistinguishable from a review session to `cleanExpiredSessions()`,
  // which deleted it on every mount, so a resized sidebar never survived a
  // restart.
  const [width, setWidth] = useState<number>(() => loadPreferences().sidebarWidth);
  const [resizing, setResizing] = useState(false);
  const widthRef = useRef(width);
  widthRef.current = width;

  const startResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setResizing(true);

    const onMove = (ev: MouseEvent) => {
      setWidth(clampSidebarWidth(ev.clientX));
    };
    const onUp = () => {
      setResizing(false);
      // Patch, don't overwrite: App owns the other fields in the blob.
      patchPreferences({ sidebarWidth: widthRef.current });
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
