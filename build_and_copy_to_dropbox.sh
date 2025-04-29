#!/bin/bash
set -euo pipefail

# Build the binary
cargo build --release

# Path to built binary
BIN="target/release/java_comment_extractor"

# Make sure it exists
if [ ! -f "$BIN" ]; then
  echo "Error: Built binary not found at $BIN"
  exit 1
fi

# Sign the binary (ad-hoc signing, preserving metadata)
codesign --sign - --force --preserve-metadata=entitlements,requirements,flags,runtime "$BIN"

# Copy to Dropbox bin directory
cp "$BIN" ~/Dropbox/bin/darwin/

echo "Build, signed, and copied to ~/Dropbox/bin/darwin successfully."
