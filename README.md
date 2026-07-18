# local-review

Local code review tool that exports AI-agent-friendly markdown. Rust port of the original TypeScript `local-review`.

## Overview

`local-review` runs inside any git repository, launches a browser-based diff viewer, and lets you add structured comments to code changes. When you're done, it exports everything as markdown with inline code context — ready to feed to an AI coding agent or share with teammates.

## Features

- Split/unified diff view with syntax highlighting (auto-detected per file)
- Line, range, file-level, and overall comments
- Comment categories: `fix`, `question`, `suggestion`, `nit`
- Collapsible sidebar with file tree, search, and filter by status
- Mark files as reviewed (auto-collapse reviewed files)
- Session persistence (JSON file backup, 14-day rolling expiry)
- Keyboard shortcuts for fast navigation
- Dark/light theme toggle
- Markdown export with code context for AI coding agents
- Virtual scrolling for large diffs
- Status bar with comment counts and review progress
- First-class TUI mode (`--tui`): syntax-highlighted diff viewer with a line
  cursor, all four comment types (line/range/file/overall) plus edit & delete,
  session persistence, 6 themes, adjustable diff context, and configurable icons
- Shared preferences (theme, icons, diff context) in
  `$XDG_CONFIG_HOME/local-code-review/config.yaml`, read by both TUI and web

## Prerequisites

- Rust toolchain (rustc, cargo) — install via [rustup](https://rustup.rs/)
- git 2.30+
- A modern browser (Chrome, Firefox, Edge)

## Installation

```bash
git clone <repo-url>
cd local-review-rs
cargo build --release
```

Optional — install binary:

```bash
cargo install --path .
```

The terminal UI (`--tui`) is built in — no extra feature flag or build step is required.

## Usage

```bash
# Default: diff HEAD vs last pushed remote commit
local-review

# Review staged changes
local-review --staged

# Review unstaged changes
local-review --unstaged

# Review all working tree changes (staged + unstaged vs HEAD)
local-review --working

# Compare two specific commits
local-review abc123 def456

# Custom port
local-review --port 3000

# Specify base branch
local-review --base origin/main

# Skip auto-opening browser
local-review --no-open

# Custom output file
local-review --output review.md

# Fetch before comparing
local-review --fetch

# Terminal TUI mode
local-review --tui
```

Output goes to stdout; server logs go to stderr. This means `local-review > review.md` works cleanly.

## CLI Reference

| Flag | Description | Default |
|------|-------------|---------|
| `<commit1> [commit2]` | Explicit commit range | — |
| `-p, --port <number>` | Server port (1–65535) | `8989` |
| `-b, --base <ref>` | Base reference for comparison | Auto-detected |
| `--staged` | Review staged changes | `false` |
| `--unstaged` | Review unstaged changes | `false` |
| `--working` | Review all working tree changes | `false` |
| `--no-open` | Don't open browser automatically | `false` |
| `-o, --output <path>` | Override output file path | `<repo>-review.md` |
| `--fetch` | Run `git fetch` before diffing | `false` |
| `--tui` | Launch terminal UI instead of server+browser | `false` |
| `-U, --context <N>` | Unified diff context lines (like `git -U<n>`); overrides config | `3` |
| `-V, --version` | Print version and exit | — |
| `--help` | Print usage and exit | — |

### TUI keys

`n`/`p` file · `↑`/`↓` line cursor · `j`/`k` comment · `c` line comment ·
`v` set range anchor (then `c`) · `F` file comment · `O` overall · `e` edit ·
`x`/`Del` delete · `r` reviewed · `s` sidebar · `d` split/unified · `t`/`T` theme ·
`+`/`-` diff context · `?` help · `q` quit (saves session + exports markdown).

### Configuration

Shared preferences live in `$XDG_CONFIG_HOME/local-code-review/config.yaml`
(falling back to `~/.config/local-code-review/config.yaml`):

```yaml
theme: default-dark        # default-dark | catppuccin-mocha | catppuccin-macchiato
                           # | catppuccin-frappe | default-light | catppuccin-latte
iconMode: nerdfont         # nerdfont | unicode | ascii
diffContextLines: 3
```

A missing or malformed file falls back to defaults. The web UI reads these via
`GET /api/v1/config`.

## Development

```bash
cargo build          # build
cargo test           # run all tests
cargo build --release  # release build
```

## Tech Stack

- **Language:** Rust
- **Server:** Axum (tokio)
- **Git:** git2 (libgit2 bindings)
- **TUI:** ratatui + crossterm
- **Frontend:** React (served as static files, identical to TS version)

## Security

- Server binds to `127.0.0.1` only — not accessible from the network
- CSRF token protection on mutating endpoints
- Path traversal prevention on file-serving routes
- No external network requests (fully offline after install)

## License

MIT
