#!/usr/bin/env bash
set -euo pipefail

# CI-safe wasm check strategy:
# 1) Prefer vendored dependencies when vendor/ exists.
# 2) Otherwise use offline check (works when cache is warm).
# 3) Optional fallback to online fetch when WASM_CHECK_ALLOW_NETWORK=1.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="wasm32-unknown-unknown"
PACKAGE="compiler-wasm"

run_check() {
  cargo check -p "$PACKAGE" --target "$TARGET" "$@"
}

if [[ -d vendor ]]; then
  echo "[wasm-check] trying vendored crates from vendor/ (offline)"
  if run_check \
    --offline \
    --config 'source.crates-io.replace-with="vendored-sources"' \
    --config 'source.vendored-sources.directory="vendor"'; then
    exit 0
  fi

  echo "[wasm-check] vendored check failed; falling back to offline cache"
fi

echo "[wasm-check] trying offline cache"
if run_check --offline; then
  echo "[wasm-check] offline cache check passed"
  exit 0
fi

if [[ "${WASM_CHECK_ALLOW_NETWORK:-0}" == "1" ]]; then
  echo "[wasm-check] offline failed; retrying with network enabled"
  run_check
  exit 0
fi

cat <<'MSG'
[wasm-check] failed in offline mode and no valid vendored tree was usable.
To make CI deterministic without network, vendor crates:
  cargo vendor vendor > .cargo/config.vendor.toml
Then run this script again.
MSG
exit 1
