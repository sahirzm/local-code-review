import { useState, useCallback, useEffect } from 'react';
import type { Comment } from '../../../shared/types.js';
import { useReviewStore } from '../hooks/useReviewStore.js';
import { downloadMarkdown } from '../utils/client-markdown.js';

const CLOSE_DELAY_SECONDS = 5;

interface SummaryPageProps {
  markdown: string;
  outputPath: string;
  csrfToken: string;
  onContinue: () => void;
}

type CategoryKey = Comment['category'];
const CATEGORIES: CategoryKey[] = ['fix', 'suggestion', 'question', 'nit'];

export function SummaryPage({ markdown, outputPath, csrfToken, onContinue }: SummaryPageProps): React.JSX.Element {
  const { comments } = useReviewStore();
  const [copied, setCopied] = useState(false);
  const [closing, setClosing] = useState(false);
  const [secondsLeft, setSecondsLeft] = useState(CLOSE_DELAY_SECONDS);
  const [showSafeToClose, setShowSafeToClose] = useState(false);

  const fileCount = new Set(comments.filter((c) => c.filePath).map((c) => c.filePath)).size;
  const counts = Object.fromEntries(CATEGORIES.map((cat) => [cat, comments.filter((c) => c.category === cat).length])) as Record<CategoryKey, number>;

  const handleDownload = useCallback(() => {
    downloadMarkdown(markdown);
  }, [markdown]);

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(markdown);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [markdown]);

  const handleClose = useCallback(async () => {
    setClosing(true);
    try {
      await fetch('/api/v1/shutdown', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrfToken },
        body: JSON.stringify({ _csrf: csrfToken }),
      });
    } catch {
      // Server may already be gone
    }
  }, [csrfToken]);

  useEffect(() => {
    if (!closing) return;
    if (secondsLeft <= 0) {
      // Browsers block window.close() on tabs the user opened directly. Try
      // anyway — works for tabs opened via JS — and otherwise fall back to a
      // "safe to close" message after a short delay.
      window.close();
      const id = setTimeout(() => setShowSafeToClose(true), 2000);
      return () => clearTimeout(id);
    }
    const id = setTimeout(() => setSecondsLeft((n) => n - 1), 1000);
    return () => clearTimeout(id);
  }, [closing, secondsLeft]);

  if (closing) {
    return (
      <div className="summary-page">
        <div className="shutdown-message" role="status">
          {showSafeToClose
            ? 'Server shut down. It is safe to close this tab.'
            : `Server shut down. Closing tab in ${secondsLeft}s...`}
        </div>
      </div>
    );
  }

  return (
    <div className="summary-page">
      <h1 className="summary-title">Review Complete</h1>
      <p className="summary-subtitle">
        Exported <strong>{comments.length}</strong> comment{comments.length !== 1 ? 's' : ''} across <strong>{fileCount}</strong> file{fileCount !== 1 ? 's' : ''}
      </p>

      <div className="summary-breakdown">
        {CATEGORIES.map((cat) => (
          counts[cat] > 0 && (
            <span key={cat} className={`category-badge category-badge-${cat}`}>
              {cat}: {counts[cat]}
            </span>
          )
        ))}
      </div>

      <p className="summary-output-path">
        Output: <code>{outputPath}</code>
      </p>

      <div className="summary-actions">
        <button className="btn btn-download" onClick={handleDownload} type="button">
          Download markdown
        </button>
        <button className="btn btn-copy" onClick={handleCopy} type="button">
          {copied ? '✓ Copied!' : 'Copy markdown'}
        </button>
        <button className="btn btn-continue" onClick={onContinue} type="button">
          Continue reviewing
        </button>
        <button className="btn btn-close" onClick={handleClose} type="button">
          Close
        </button>
      </div>
    </div>
  );
}
