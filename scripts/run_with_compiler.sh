#!/usr/bin/env bash
set -euo pipefail

# This script builds the Rust `dsl-compiler` (release) and then starts the
# web application using a Python virtual environment inside `web_service/app`.
# Place this script in the repo `scripts/` folder and run it from the repo root:
#   ./scripts/run_with_compiler.sh

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DSL_DIR="$REPO_ROOT/dsl"
WEB_DIR="$REPO_ROOT/web_service/app"

FORCE_FALLBACK=0
if [ "${1:-}" = "--force-fallback" ]; then
  FORCE_FALLBACK=1
fi

echo "[run_with_compiler] repo root: $REPO_ROOT"

if [ ! -d "$DSL_DIR" ]; then
  echo "DSL directory not found: $DSL_DIR" >&2
  exit 1
fi

cd "$DSL_DIR"
echo "[run_with_compiler] Building dsl-compiler (release)..."
mkdir -p "$REPO_ROOT/logs"
BUILD_LOG="$REPO_ROOT/logs/compiler-build.log"
echo "[run_with_compiler] Logging build output to $BUILD_LOG"
if ! cargo build --release > "$BUILD_LOG" 2>&1; then
  echo "[run_with_compiler] cargo build --release failed. See $BUILD_LOG for details." >&2
  if [ "$FORCE_FALLBACK" -ne 1 ]; then
    echo "[run_with_compiler] Aborting. Pass --force-fallback to continue without real compiler." >&2
    exit 2
  else
    echo "[run_with_compiler] --force-fallback enabled; continuing without release binary." >&2
  fi
fi

BIN="$DSL_DIR/target/release/dsl-compiler"
if [ ! -x "$BIN" ]; then
  REP_BIN="$REPO_ROOT/target/release/dsl-compiler"
  if [ -x "$REP_BIN" ]; then
    BIN="$REP_BIN"
  fi
fi

if [ -x "$BIN" ]; then
  echo "[run_with_compiler] Found release binary: $BIN"
else
  echo "[run_with_compiler] Release binary not found after build: $BIN" >&2
  if [ "$FORCE_FALLBACK" -ne 1 ]; then
    echo "[run_with_compiler] Aborting because real compiler is required. Use --force-fallback to continue without it." >&2
    exit 3
  else
    echo "[run_with_compiler] --force-fallback enabled; starting server using sample parser as fallback." >&2
  fi
fi

if [ ! -d "$WEB_DIR" ]; then
  echo "Web app directory not found: $WEB_DIR" >&2
  exit 1
fi

cd "$WEB_DIR"

# Create a local venv if missing
if [ ! -d ".venv" ]; then
  echo "[run_with_compiler] Creating Python virtualenv at $WEB_DIR/.venv"
  python3 -m venv .venv
fi

echo "[run_with_compiler] Activating venv and installing dependencies"
. .venv/bin/activate
pip install --upgrade pip
# Install uvicorn with WebSocket support
pip install fastapi "uvicorn[standard]"

echo "[run_with_compiler] Starting uvicorn (web app) — this replaces the current shell"
exec .venv/bin/uvicorn main:app --reload --host 127.0.0.1 --port 8000
