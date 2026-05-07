#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 Alexander Mohr

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${SCRIPT_DIR}/slides"

mkdir -p "$OUTPUT_DIR"

bunx @marp-team/marp-cli "$SCRIPT_DIR/slides.md" \
  --html \
  --output "$OUTPUT_DIR/slides.html"

bunx @marp-team/marp-cli "$SCRIPT_DIR/slides.md" \
  --html \
  --pdf \
  --output "$OUTPUT_DIR/slides.pdf"

echo "Slides built in $OUTPUT_DIR"
