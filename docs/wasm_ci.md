# WASM CI Check Strategy

Use `scripts/ci_wasm_check.sh` for deterministic wasm compile checks.

## Modes

1. Vendored mode (recommended for CI):
   - Put vendored crates in `vendor/`.
   - The script automatically runs Cargo with vendored sources and `--offline`.

2. Offline-cache mode:
   - If `vendor/` does not exist, the script tries `cargo check --offline`.
   - This works when the CI cache already contains all required crates.

3. Network fallback (optional):
   - Set `WASM_CHECK_ALLOW_NETWORK=1` to allow one online retry.

## Setup vendored crates

```bash
cargo vendor vendor > .cargo/config.vendor.toml
```

Then run:

```bash
scripts/ci_wasm_check.sh
```
