/// <reference types="@vitest/browser/context" />
/// <reference types="@vitest/browser/matchers" />
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-react';
import { SummaryPage } from './SummaryPage.js';
import { ReviewStoreProvider } from '../hooks/useReviewStore.js';
import { mockApi, type MockApiHandle } from '../test/mock-api.js';

function renderSummary(): ReturnType<typeof render> {
  return render(
    <ReviewStoreProvider>
      <SummaryPage markdown="# review" outputPath="/tmp/out.md" csrfToken="t" onContinue={() => {}} />
    </ReviewStoreProvider>,
  );
}

describe('SummaryPage close countdown (browser)', () => {
  let api: MockApiHandle;
  let originalClose: typeof window.close;

  beforeEach(() => {
    localStorage.clear();
    api = mockApi();
    originalClose = window.close;
  });

  afterEach(() => {
    window.close = originalClose;
    api.restore();
  });

  it('starts the countdown at 3s and calls window.close()', async () => {
    const closeSpy = vi.fn();
    window.close = closeSpy;

    const screen = renderSummary();
    await screen.getByRole('button', { name: 'Close' }).click();

    // Countdown begins at the reduced 3-second delay.
    await expect.element(screen.getByText(/Closing tab in 3s/)).toBeInTheDocument();

    // Within the countdown window window.close() is attempted.
    await vi.waitFor(() => expect(closeSpy).toHaveBeenCalled(), { timeout: 6000 });
  });
});
