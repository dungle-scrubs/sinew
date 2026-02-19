#!/usr/bin/env bash
set -euo pipefail

echo "Installing git hooks (.githooks)..."
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push .githooks/install.sh

echo "Git hooks installed."
echo "- pre-commit: cargo fmt/clippy/check"
echo "- pre-push: trufflehog secret scan"
