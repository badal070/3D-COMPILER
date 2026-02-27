#!/usr/bin/env bash
set -euo pipefail

# Fresh launcher for edu3d modeling service.
# - Builds dsl-compiler (release)
# - Creates/uses isolated venv under web_service/.venv
# - Installs web_service/requirements.txt
# - Starts uvicorn using package import path (app.main:app)
#
# Usage:
#   ./scripts/run_with_compiler.sh

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB_ROOT="$REPO_ROOT/web_service"
WEB_APP_DIR="$WEB_ROOT/app"
VENV_DIR="$WEB_ROOT/.venv"
LEGACY_VENV_DIR="$WEB_APP_DIR/.venv"
LOG_DIR="$REPO_ROOT/logs"
BUILD_LOG="$LOG_DIR/compiler-build.log"

PY_BIN="${PY_BIN:-python3}"
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-8000}"

echo "[run_with_compiler] repo root: $REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[run_with_compiler] cargo not found in PATH" >&2
  exit 1
fi

if ! command -v "$PY_BIN" >/dev/null 2>&1; then
  echo "[run_with_compiler] python executable not found: $PY_BIN" >&2
  exit 1
fi

if [ ! -d "$WEB_ROOT" ] || [ ! -d "$WEB_APP_DIR" ]; then
  echo "[run_with_compiler] web_service/app not found under $REPO_ROOT" >&2
  exit 1
fi

if [ -d "$LEGACY_VENV_DIR" ]; then
  VENV_DIR="$LEGACY_VENV_DIR"
fi

mkdir -p "$LOG_DIR"

echo "[run_with_compiler] Building dsl-compiler (release)..."
echo "[run_with_compiler] Logging build output to $BUILD_LOG"
if ! (cd "$REPO_ROOT" && cargo build --release -p dsl-compiler >"$BUILD_LOG" 2>&1); then
  echo "[run_with_compiler] cargo build failed. See: $BUILD_LOG" >&2
  exit 2
fi

BIN_CANDIDATES=(
  "$REPO_ROOT/target/release/dsl-compiler"
  "$REPO_ROOT/dsl/target/release/dsl-compiler"
)

DSL_BIN=""
for candidate in "${BIN_CANDIDATES[@]}"; do
  if [ -x "$candidate" ]; then
    DSL_BIN="$candidate"
    break
  fi
done

if [ -z "$DSL_BIN" ]; then
  echo "[run_with_compiler] release binary not found after build" >&2
  exit 3
fi
echo "[run_with_compiler] Found release binary: $DSL_BIN"

if [ ! -d "$VENV_DIR" ]; then
  echo "[run_with_compiler] Creating virtual environment: $VENV_DIR"
  "$PY_BIN" -m venv "$VENV_DIR"
fi

echo "[run_with_compiler] Activating venv and installing dependencies"
# shellcheck disable=SC1090
source "$VENV_DIR/bin/activate"
if ! python -m pip install -r "$WEB_ROOT/requirements.txt"; then
  echo "[run_with_compiler] Dependency install failed (possibly offline). Checking existing environment..." >&2
  if ! python -c "import fastapi, uvicorn, pydantic" >/dev/null 2>&1; then
    echo "[run_with_compiler] Required runtime packages are not available in $VENV_DIR" >&2
    echo "[run_with_compiler] Connect to network or preinstall: fastapi uvicorn pydantic" >&2
    exit 4
  fi
  echo "[run_with_compiler] Using existing installed packages in $VENV_DIR"
fi

cd "$WEB_ROOT"
export PYTHONPATH="$WEB_ROOT${PYTHONPATH:+:$PYTHONPATH}"

echo "[run_with_compiler] Starting uvicorn on http://$HOST:$PORT"
echo "[run_with_compiler] Import target: app.main:app"
exec "$VENV_DIR/bin/uvicorn" \
  app.main:app \
  --reload \
  --host "$HOST" \
  --port "$PORT" \
  --reload-dir "$WEB_APP_DIR"
