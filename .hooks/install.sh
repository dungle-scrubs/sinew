#!/bin/bash
# Install git hooks for this repository
# Run this script after cloning: .hooks/install.sh

set -e

echo "Installing git hooks..."

# Configure git to use the .hooks directory
git config core.hooksPath .hooks

echo "Git hooks installed successfully!"
echo ""
echo "Pre-commit hook will run:"
echo "  1. cargo fmt --check  (formatting)"
echo "  2. cargo clippy       (linting)"
echo "  3. cargo check        (type checking)"
