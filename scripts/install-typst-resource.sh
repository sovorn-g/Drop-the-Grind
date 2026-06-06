#!/usr/bin/env bash
set -euo pipefail

# Install Typst macOS binary as a bundled app resource.
# Usage: ./scripts/install-typst-resource.sh
# Pins v0.13.1 — update version here when upgrading.

VERSION="v0.13.1"
ARCH="aarch64-apple-darwin"
TARBALL="typst-${ARCH}.tar.xz"
URL="https://github.com/typst/typst/releases/download/${VERSION}/${TARBALL}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RESOURCES_DIR="${SCRIPT_DIR}/../src-tauri/resources"
BIN_DIR="${RESOURCES_DIR}/bin"

if [ -f "${BIN_DIR}/typst" ]; then
    INSTALLED_VER="$("${BIN_DIR}/typst" --version 2>/dev/null | head -1)"
    echo "Typst already installed: ${INSTALLED_VER}"
    echo "To reinstall, remove ${BIN_DIR}/typst and re-run this script."
    exit 0
fi

echo "Downloading Typst ${VERSION} (${ARCH})..."
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

curl -fsSL "${URL}" -o "${TMP_DIR}/${TARBALL}"

echo "Extracting..."
tar -xf "${TMP_DIR}/${TARBALL}" -C "${TMP_DIR}"

mkdir -p "${BIN_DIR}"
cp "${TMP_DIR}/typst-${ARCH}/typst" "${BIN_DIR}/typst"
chmod +x "${BIN_DIR}/typst"

echo "Typst installed at ${BIN_DIR}/typst"
"${BIN_DIR}/typst" --version
