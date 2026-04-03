#!/usr/bin/env bash
set -euo pipefail

# Regenerate src/bindings.rs from the librdb C headers.
# Requires: cargo install bindgen-cli
#
# Usage:
#   cd librdb-sys
#   ./update-bindings.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LIBRDB_ROOT="$SCRIPT_DIR/librdb"
WRAPPER_H="$SCRIPT_DIR/wrapper.h"
OUTPUT="$SCRIPT_DIR/src/bindings.rs"

if ! command -v bindgen &>/dev/null; then
    echo "error: bindgen-cli not found. Install with: cargo install bindgen-cli" >&2
    exit 1
fi

bindgen "$WRAPPER_H" \
    -I "$LIBRDB_ROOT/api" \
    -I "$LIBRDB_ROOT/src" \
    -I "$LIBRDB_ROOT/deps/redis" \
    --allowlist-function 'RDB_.*' \
    --allowlist-function 'RDBX_.*' \
    --allowlist-type 'Rdb.*' \
    --allowlist-type 'Rdbx.*' \
    --allowlist-var 'RDB_.*' \
    --allowlist-var 'RDBX_.*' \
    --with-derive-debug \
    --with-derive-default \
    -o "$OUTPUT"

rustfmt "$OUTPUT"

echo "Generated $OUTPUT"
