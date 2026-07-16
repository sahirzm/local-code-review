#!/usr/bin/env bash
# Backend (Rust) test-coverage report.
#
# cargo-llvm-cov does not build in this environment, so we drive LLVM's
# source-based coverage directly via the llvm-tools-preview binaries that ship
# with the toolchain. Requires: rustup component add llvm-tools-preview.
#
# The rustup stable toolchain (1.96.0) fails to link proc-macro crates here, so
# the build runs under `nix-shell` (see shell.nix) which provides a working
# rustc/cargo plus the openssl/pkg-config native deps.
set -euo pipefail

cd "$(dirname "$0")/.."

PROFRAW_DIR="${PROFRAW_DIR:-target/coverage}"
rm -rf "$PROFRAW_DIR"
mkdir -p "$PROFRAW_DIR"

echo "==> Running instrumented tests"
nix-shell -p rustc cargo pkg-config openssl zlib --run "
  RUSTFLAGS='-C instrument-coverage' \
  LLVM_PROFILE_FILE='$PWD/$PROFRAW_DIR/cov-%p-%m.profraw' \
  cargo test --bin local-review
"

# llvm-profdata / llvm-cov ship inside the rustup toolchain sysroot.
TOOLS_BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
PROFDATA="$TOOLS_BIN/llvm-profdata"
LLVMCOV="$TOOLS_BIN/llvm-cov"

echo "==> Merging profiles"
"$PROFDATA" merge -sparse "$PROFRAW_DIR"/*.profraw -o "$PROFRAW_DIR/merged.profdata"

BIN="$(find target/debug/deps -name 'local_review-*' ! -name '*.d' -type f | head -1)"

echo "==> Coverage report"
"$LLVMCOV" report "$BIN" \
  -instr-profile="$PROFRAW_DIR/merged.profdata" \
  -ignore-filename-regex='(/registry/|/rustc/|\.cargo/)'

# Emit a browsable HTML report alongside the summary.
"$LLVMCOV" show "$BIN" \
  -instr-profile="$PROFRAW_DIR/merged.profdata" \
  -ignore-filename-regex='(/registry/|/rustc/|\.cargo/)' \
  -format=html -output-dir=target/coverage/html >/dev/null
echo "==> HTML report: target/coverage/html/index.html"
