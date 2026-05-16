import { useState, useCallback } from 'react';
import type { Comment } from '../../../shared/types.js';
import { useReviewStore } from '../hooks/useReviewStore.js';
import { downloadMarkdown } from '../utils/client-markdown.js';

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
  const [shutdownMsg, setShutdownMsg] = useState('');

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
    setShutdownMsg('Server shutting down...');
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

  if (shutdownMsg) {
    return (
      <div className="summary-page">
        <div className="shutdown-message" role="status">{shutdownMsg}</div>
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
