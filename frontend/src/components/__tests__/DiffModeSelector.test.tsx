import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DiffModeSelector, type DiffModeResult } from '../DiffModeSelector.js';

const result: DiffModeResult = {
  metadata: {
    repoName: 'demo',
    commitRange: 'staged',
    baseRef: 'staged',
    headRef: 'HEAD',
    files: [],
    timestamp: 't',
    csrfToken: 'tok',
  },
  diff: { files: [] },
};

describe('DiffModeSelector', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('posts the chosen mode and reports the fresh diff', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(result),
    });
    vi.stubGlobal('fetch', fetchMock);
    const onSwitched = vi.fn();

    render(<DiffModeSelector csrfToken="tok" onSwitched={onSwitched} onError={() => undefined} />);
    fireEvent.change(screen.getByLabelText('Diff base'), { target: { value: 'staged' } });

    await waitFor(() => expect(onSwitched).toHaveBeenCalledWith(result));
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/v1/diff-mode');
    expect(JSON.parse((init as RequestInit).body as string)).toEqual({ mode: 'staged' });
    expect((init as RequestInit).headers).toMatchObject({ 'X-CSRF-Token': 'tok' });
  });

  it('reports an error when the server rejects the switch', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: () => Promise.resolve({ error: 'Unknown diff mode: bogus' }),
    });
    vi.stubGlobal('fetch', fetchMock);
    const onError = vi.fn();

    render(<DiffModeSelector csrfToken="tok" onSwitched={() => undefined} onError={onError} />);
    fireEvent.change(screen.getByLabelText('Diff base'), { target: { value: 'staged' } });

    await waitFor(() => expect(onError).toHaveBeenCalledWith('Unknown diff mode: bogus'));
  });
});
