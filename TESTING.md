# Testing & Coverage

This project has two independently-tested halves:

- **Frontend** (`frontend/`) — React + TypeScript, tested with **Vitest**.
- **Backend** (`src/`) — Rust (axum server + git integration + markdown export),
  tested with `cargo test`.

## Frontend

### Running tests

```bash
cd frontend
npm test                 # both projects: jsdom unit + headless-Chromium browser
npm run coverage         # unit-project coverage (text + HTML in frontend/coverage/)
```

Vitest is configured with two projects (`frontend/vitest.workspace.ts`):

| Project   | Environment            | Files                     | Purpose                                             |
|-----------|------------------------|---------------------------|-----------------------------------------------------|
| `unit`    | jsdom                  | `src/**/*.{test,spec}.*`  | Component logic, hooks, and pure utilities          |
| `browser` | real headless Chromium | `src/**/*.browser.test.tsx` | End-to-end feature behavior (themes, font size, icons, close flow) |

The `unit` project auto-unmounts React trees between tests via
`src/test/setup-unit.ts` (Testing Library `cleanup` + `jest-dom` matchers).

Browser tests need a one-time Chromium install:

```bash
npm run test:browser:install
```

### What is covered

`npm test` runs **130 tests** (116 unit + 14 browser). Unit-project coverage:

| Area                         | Coverage highlights                                        |
|------------------------------|------------------------------------------------------------|
| `utils/` (build-file-tree, transform-diff, client-markdown, file-icon) | 96% statements |
| `hooks/` (useReviewStore, useSession, useKeyboardShortcuts, useQuotaMonitor) | 84% statements |
| `components/` (CommentForm, CommentWidget, OverallComments, DiffView, FileDiff) | logic paths covered |
| `themes.ts`                  | 100%                                                       |

`App.tsx`, `Sidebar.tsx`, and `SummaryPage.tsx` show as 0% in the **unit**
coverage report because they are exercised by the **browser** project (real
Chromium), which the v8 coverage provider does not instrument. Their feature
behavior is covered by `*.browser.test.tsx`. `types.ts` and `main.tsx` are
type-only / entry-point files with no branch logic.

## Backend (Rust)

### Running tests

```bash
# The rustup stable toolchain here fails to link proc-macro crates, so use the
# nix-provided toolchain (see shell.nix). It also supplies openssl/pkg-config.
nix-shell -p rustc cargo pkg-config openssl zlib --run "cargo test --bin local-review"
```

`cargo test` runs **114 tests** across the git parser, range resolution,
markdown export, CSRF/path-guard middleware, shutdown coordination, CLI parsing,
session persistence, and the HTTP API routes.

### Coverage

cargo-llvm-cov does not compile in this environment, so `scripts/coverage.sh`
drives LLVM source-based coverage directly using the `llvm-tools-preview`
binaries:

```bash
rustup component add llvm-tools-preview   # one-time
./scripts/coverage.sh                     # summary + HTML in target/coverage/html/
```

Backend coverage totals **69.9% region / 70.3% line**. The server API and
session layers — the parts that were previously untested — are now well covered:

| Module                    | Region cover | Line cover |
|---------------------------|-------------:|-----------:|
| `session/mod.rs`          | 96.5%        | 98.0%      |
| `server/routes/mod.rs`    | 94.6%        | 92.2%      |
| `git/diff_parser.rs`      | 98.7%        | 98.6%      |
| `output/markdown.rs`      | 99.2%        | 99.1%      |
| `cli/mod.rs`              | 98.0%        | 97.5%      |
| `server/middleware/*`     | 97–100%      | 97–100%    |

The remaining uncovered code is the **TUI** (`src/tui/**`) — an interactive
terminal renderer that requires a live terminal — and the server/process
bootstrap (`server/mod.rs`, `server/frontend.rs`, `main.rs`), which bind sockets
and spawn the real process. These are integration/manual-test surfaces rather
than unit-testable logic.
