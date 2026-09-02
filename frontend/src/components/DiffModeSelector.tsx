import { useState, useCallback } from 'react';
import { GitCompare } from 'lucide-react';
import type { ReviewMetadata, DiffResponse } from '../shared/types.js';

/** Diff modes the runtime switch supports. Mirrors the CLI selection. */
export type DiffMode = 'working' | 'staged' | 'unstaged' | 'all' | 'last_pushed' | 'commits';

const MODE_LABELS: { value: DiffMode; label: string }[] = [
  { value: 'working', label: 'Working tree (vs HEAD)' },
  { value: 'staged', label: 'Staged' },
  { value: 'unstaged', label: 'Unstaged' },
  { value: 'all', label: 'All changes (vs last pushed)' },
  { value: 'last_pushed', label: 'Last pushed..HEAD' },
];

export interface DiffModeResult {
  metadata: ReviewMetadata;
  diff: DiffResponse;
}

interface DiffModeSelectorProps {
  csrfToken: string;
  /** Called with the fresh metadata + diff after a successful switch. */
  onSwitched: (result: DiffModeResult) => void;
  onError: (message: string) => void;
}

/**
 * Runtime diff-base selector. Changes what the review targets (staged, working,
 * a branch/commit range, etc.) without restarting the server: POSTs the chosen
 * mode to /api/v1/diff-mode and hands the recomputed metadata + diff back to
 * the caller to re-render.
 */
export function DiffModeSelector({ csrfToken, onSwitched, onError }: DiffModeSelectorProps): React.JSX.Element {
  const [mode, setMode] = useState<DiffMode>('working');
  const [busy, setBusy] = useState(false);

  const switchMode = useCallback(
    async (next: DiffMode) => {
      setMode(next);
      setBusy(true);
      try {
        const res = await fetch('/api/v1/diff-mode', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrfToken },
          body: JSON.stringify({ mode: next }),
        });
        if (!res.ok) {
          const detail = await res.json().catch(() => ({}));
          throw new Error((detail as { error?: string }).error ?? `diff-mode: ${res.status}`);
        }
        const result = (await res.json()) as DiffModeResult;
        onSwitched(result);
      } catch (err) {
        onError(err instanceof Error ? err.message : 'Failed to switch diff mode');
      } finally {
        setBusy(false);
      }
    },
    [csrfToken, onSwitched, onError],
  );

  return (
    <div className="diff-mode-selector">
      <span className="toolbar-nav-icon" aria-hidden="true"><GitCompare size={15} /></span>
      <select
        className="context-select"
        value={mode}
        disabled={busy}
        onChange={(e) => switchMode(e.target.value as DiffMode)}
        aria-label="Diff base"
        title="Change what this review compares"
      >
        {MODE_LABELS.map((m) => (
          <option key={m.value} value={m.value}>{m.label}</option>
        ))}
      </select>
    </div>
  );
}
