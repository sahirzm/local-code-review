#!/usr/bin/env bash
set -euo pipefail

NO_INSTALL=false
for arg in "$@"; do
	if [ "$arg" = "--no-install" ]; then
		NO_INSTALL=true
	fi
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building local-review (Rust) ==="

if ! command -v cargo &>/dev/null; then
	echo "Error: cargo not found. Install Rust: https://rustup.rs/"
	exit 1
fi

echo ""
echo "--- Step 1: Frontend ---"
FRONTEND_DIR="$SCRIPT_DIR/frontend"
if [ -f "$FRONTEND_DIR/package.json" ]; then
	cd "$FRONTEND_DIR"
	if [ ! -d "node_modules" ]; then
		echo "Installing frontend dependencies..."
		npm install
	fi
	echo "Building frontend with Vite..."
	npx vite build
	echo "Frontend built to: $FRONTEND_DIR/dist"
else
	echo "Warning: frontend/package.json not found, skipping frontend build"
fi

cd "$SCRIPT_DIR"

echo ""
echo "--- Step 2: Rust release build ---"
echo "Building release binary..."
cargo build --release

BINARY="$SCRIPT_DIR/target/release/local-review"
if [ -f "$BINARY" ]; then
	echo ""
	echo "=== Build complete ==="
	echo "Binary: $BINARY"
	ls -lh "$BINARY"
	echo ""
	echo "Usage examples:"
	echo "  $BINARY                    # Review HEAD vs last pushed"
	echo "  $BINARY --staged            # Review staged changes"
	echo "  $BINARY --unstaged          # Review unstaged changes"
	echo "  $BINARY --working           # Review all working tree changes"
	echo "  $BINARY --port 3000         # Custom port"
	echo "  $BINARY --no-open           # Don't open browser"
	echo ""
	echo ""
	echo "--- Step 3: Install ---"
	if [ "$NO_INSTALL" = true ]; then
		echo "Skipping install (--no-install passed)"
	else
		echo "Installing binary with cargo install..."
		cargo install --path .
		echo "Installed successfully"
	fi
else
	echo "Error: Build failed"
	exit 1
fi
