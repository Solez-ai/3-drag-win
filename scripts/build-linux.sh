#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════
# build-linux.sh - Build 3-win-drag Linux release packages
#
# Usage:
#   ./scripts/build-linux.sh              # native build via cargo
#   ./scripts/build-linux.sh cross         # cross-compile via cross + Docker
#   TARGET=aarch64-unknown-linux-gnu ./scripts/build-linux.sh  # ARM64
# ═══════════════════════════════════════════════════════════════════════

set -euo pipefail
cd "$(dirname "$0")/.."

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
MODE="${1:-native}"
PROFILE="${PROFILE:-release}"
BIN_NAME="3-win-drag"
PKG_NAME="${BIN_NAME}-${TARGET}"

echo "═══ Building ${BIN_NAME} for ${TARGET} (${MODE}) ═══"

# ── Build ──────────────────────────────────────────────────────
case "$MODE" in
  native)
    cargo build --target "$TARGET" --"$PROFILE"
    ;;
  cross)
    if ! command -v cross &>/dev/null; then
      echo "Installing cross ..."
      cargo install cross
    fi
    cross build --target "$TARGET" --"$PROFILE"
    ;;
  *)
    echo "Unknown mode: $MODE  (use 'native' or 'cross')"
    exit 1
    ;;
esac

# ── Package ─────────────────────────────────────────────────────
echo "═══ Packaging ${PKG_NAME} ═══"

rm -rf "target/${PKG_NAME}"
mkdir -p "target/${PKG_NAME}"

cp "target/${TARGET}/${PROFILE}/${BIN_NAME}" "target/${PKG_NAME}/"
cp README.md LICENSE "target/${PKG_NAME}/"

cd target
tar czf "${PKG_NAME}.tar.gz" "${PKG_NAME}"
echo "═══ Done → target/${PKG_NAME}.tar.gz ═══"
