#!/usr/bin/env bash
# Install local-review as an MCP server and register it with any supported AI
# coding agents found on this machine.
#
# Supported agents: Claude Code, Kiro CLI, opencode, and pi.dev (via the
# pi-mcp-adapter package). Registration is idempotent — re-running updates the
# existing entry.
#
# Usage:
#   scripts/install-mcp.sh [--scope user|project] [--no-build]
#
#   --scope user      Register in the user/global config (default).
#   --scope project   Register in the current repo's project config.
#   --no-build        Skip building; use an already-installed `local-review`.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

SCOPE="user"
DO_BUILD=true
while [ $# -gt 0 ]; do
	case "$1" in
		--scope) SCOPE="${2:-user}"; shift 2 ;;
		--no-build) DO_BUILD=false; shift ;;
		-h|--help) sed -n '2,17p' "$0"; exit 0 ;;
		*) echo "Unknown argument: $1" >&2; exit 1 ;;
	esac
done

case "$SCOPE" in
	user|project) ;;
	*) echo "Invalid scope: $SCOPE (expected 'user' or 'project')" >&2; exit 1 ;;
esac

log() { printf '\033[1m%s\033[0m\n' "$*"; }
info() { printf '  %s\n' "$*"; }

# --- 1. Resolve the binary path -------------------------------------------------
BIN=""
if [ "$DO_BUILD" = true ]; then
	log "Building local-review (release)..."
	( cd "$REPO_DIR" && ./build.sh --no-install )
	BIN="$REPO_DIR/target/release/local-review"
elif command -v local-review &>/dev/null; then
	BIN="$(command -v local-review)"
elif [ -x "$REPO_DIR/target/release/local-review" ]; then
	BIN="$REPO_DIR/target/release/local-review"
else
	echo "Error: local-review binary not found. Run without --no-build, or 'cargo install --path .'." >&2
	exit 1
fi
log "Using binary: $BIN"

# JSON snippet shared by every agent: how to launch the server.
# command=<abs path>, args=["mcp"].

# --- 2. Helper: merge one mcpServers entry into a JSON config file --------------
# Args: <config-file> <json-pointer-parent-key> <server-name>
# Uses python3 (present on macOS/Linux dev boxes) for a safe JSON merge.
merge_json() {
	local file="$1" parent="$2" name="$3"
	mkdir -p "$(dirname "$file")"
	[ -f "$file" ] || echo '{}' > "$file"
	BIN="$BIN" PARENT="$parent" NAME="$name" FILE="$file" python3 - <<'PY'
import json, os
file, parent, name, binp = os.environ["FILE"], os.environ["PARENT"], os.environ["NAME"], os.environ["BIN"]
with open(file) as f:
    try:
        data = json.load(f)
    except json.JSONDecodeError:
        data = {}
data.setdefault(parent, {})
data[parent][name] = {"command": binp, "args": ["mcp"]}
with open(file, "w") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
PY
}

REGISTERED=0

# --- 3. Claude Code -------------------------------------------------------------
if command -v claude &>/dev/null; then
	log "Registering with Claude Code..."
	if [ "$SCOPE" = "project" ]; then
		claude mcp add --scope project local-review -- "$BIN" mcp
	else
		claude mcp add --scope user local-review -- "$BIN" mcp
	fi
	info "done (claude mcp add)"
	REGISTERED=$((REGISTERED + 1))
else
	info "Claude Code (claude) not found — skipping."
fi

# --- 4. Kiro CLI ----------------------------------------------------------------
if command -v kiro-cli &>/dev/null || command -v kiro &>/dev/null; then
	log "Registering with Kiro CLI..."
	if [ "$SCOPE" = "project" ]; then
		KIRO_CFG=".kiro/settings/mcp.json"
	else
		KIRO_CFG="$HOME/.kiro/settings/mcp.json"
	fi
	merge_json "$KIRO_CFG" "mcpServers" "local-review"
	info "wrote $KIRO_CFG"
	REGISTERED=$((REGISTERED + 1))
else
	info "Kiro CLI (kiro-cli) not found — skipping."
fi

# --- 5. opencode ----------------------------------------------------------------
if command -v opencode &>/dev/null; then
	log "Registering with opencode..."
	if [ "$SCOPE" = "project" ]; then
		OC_CFG="opencode.json"
	else
		OC_CFG="$HOME/.config/opencode/opencode.json"
	fi
	# opencode uses type=local + command array; write it directly.
	mkdir -p "$(dirname "$OC_CFG")"
	[ -f "$OC_CFG" ] || echo '{}' > "$OC_CFG"
	BIN="$BIN" FILE="$OC_CFG" python3 - <<'PY'
import json, os
file, binp = os.environ["FILE"], os.environ["BIN"]
with open(file) as f:
    try:
        data = json.load(f)
    except json.JSONDecodeError:
        data = {}
data.setdefault("$schema", "https://opencode.ai/config.json")
data.setdefault("mcp", {})
data["mcp"]["local-review"] = {"type": "local", "command": [binp, "mcp"], "enabled": True}
with open(file, "w") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
PY
	info "wrote $OC_CFG"
	REGISTERED=$((REGISTERED + 1))
else
	info "opencode not found — skipping."
fi

# --- 6. pi.dev (via pi-mcp-adapter) ---------------------------------------------
if command -v pi &>/dev/null; then
	log "Configuring pi.dev (via pi-mcp-adapter)..."
	if [ "$SCOPE" = "project" ]; then
		PI_CFG=".pi/settings.json"
	else
		PI_CFG="$HOME/.pi/agent/settings.json"
	fi
	# pi has no native MCP; pi-mcp-adapter bridges a stdio MCP server. This writes
	# an mcpServers entry the adapter consumes. See https://pi.dev/packages/pi-mcp-adapter
	merge_json "$PI_CFG" "mcpServers" "local-review"
	info "wrote $PI_CFG"
	info "Ensure the pi-mcp-adapter package is installed: pi package add pi-mcp-adapter"
	REGISTERED=$((REGISTERED + 1))
else
	info "pi.dev (pi) not found — skipping."
fi

echo ""
if [ "$REGISTERED" -eq 0 ]; then
	log "No supported agents detected. Manual config snippets are in README.md."
else
	log "Registered local-review with $REGISTERED agent(s)."
	info "Ask your agent to call the 'start_review' tool to try it."
fi
