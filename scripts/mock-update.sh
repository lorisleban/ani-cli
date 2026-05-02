#!/usr/bin/env bash
# mock-update.sh — set up a local fake release so `ani-cli` can exercise
# the self-update flow without hitting GitHub.
#
# Usage:
#   ./scripts/mock-update.sh               # defaults to macos-aarch64
#   ./scripts/mock-update.sh linux-x86_64
#   ./scripts/mock-update.sh macos-aarch64
#   ./scripts/mock-update.sh macos-x86_64
#   ./scripts/mock-update.sh windows-x86_64
#
# After running, it prints environment variables. Launch ani-cli with those
# vars to test the update check + upgrade path:
#
#   eval $(./scripts/mock-update.sh)
#   cargo run
#
# The fake release declares version v999.0.0, so the check will always find
# an "update available". The asset archive contains a dummy binary that
# replaces the running one (on Unix it is a copy of the current binary; on
# CI where no binary exists yet, it is an empty executable stub).

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TEST_DIR="$ROOT_DIR/.update-test"

PLATFORM="${1:-macos-aarch64}"

case "$PLATFORM" in
  macos-aarch64|macos-x86_64|linux-x86_64)
    ARCHIVE_NAME="ani-cli-${PLATFORM}.tar.gz"
    BINARY_NAME="ani-cli"
    ;;
  windows-x86_64)
    ARCHIVE_NAME="ani-cli-windows-x86_64.zip"
    BINARY_NAME="ani-cli.exe"
    ;;
  *)
    echo "unsupported platform: $PLATFORM" >&2
    echo "supported: macos-aarch64 macos-x86_64 linux-x86_64 windows-x86_64" >&2
    exit 1
    ;;
esac

mkdir -p "$TEST_DIR/stage"

# Build a fake binary — prefer the current debug binary, fall back to a stub.
if [ -f "$ROOT_DIR/target/debug/ani-cli" ]; then
  cp "$ROOT_DIR/target/debug/ani-cli" "$TEST_DIR/stage/$BINARY_NAME"
else
  cat > "$TEST_DIR/stage/$BINARY_NAME" <<'STUB'
#!/bin/sh
echo "mock-updated ani-cli v999.0.0"
STUB
  chmod +x "$TEST_DIR/stage/$BINARY_NAME"
fi

# Create the archive from the staged binary.
rm -f "$TEST_DIR/$ARCHIVE_NAME"
case "$ARCHIVE_NAME" in
  *.tar.gz)
    tar -czf "$TEST_DIR/$ARCHIVE_NAME" -C "$TEST_DIR/stage" "$BINARY_NAME"
    ;;
  *.zip)
    (cd "$TEST_DIR/stage" && zip -j "$TEST_DIR/$ARCHIVE_NAME" "$BINARY_NAME")
    ;;
esac

# Write the fake release JSON.
cat > "$TEST_DIR/release.json" <<EOF
{
  "tag_name": "v999.0.0",
  "html_url": "https://example.com/release-notes",
  "assets": [
    {
      "name": "$ARCHIVE_NAME",
      "browser_download_url": "file://$TEST_DIR/$ARCHIVE_NAME"
    }
  ]
}
EOF

# Print the env vars the app reads.
echo ""
echo "# ── paste these into your shell before running ani-cli ──"
echo "export ANI_CLI_UPDATE_URL=\"file://$TEST_DIR/release.json\""
echo "export ANI_CLI_UPDATE_ASSET_PATH=\"$TEST_DIR/$ARCHIVE_NAME\""
echo "export ANI_CLI_UPDATE_SOURCE=\"direct\""
echo ""
echo "# or eval:"
echo "#   eval \$(./scripts/mock-update.sh $PLATFORM)"
