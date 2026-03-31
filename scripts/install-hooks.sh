#!/bin/bash

# Install git hooks from scripts/ into .git/hooks/
# Run once after cloning: bash scripts/install-hooks.sh

HOOKS_DIR="$(git rev-parse --show-toplevel)/.git/hooks"
SCRIPTS_DIR="$(git rev-parse --show-toplevel)/scripts"

for hook in pre-commit; do
  if [ -f "$SCRIPTS_DIR/$hook" ]; then
    cp "$SCRIPTS_DIR/$hook" "$HOOKS_DIR/$hook"
    chmod +x "$HOOKS_DIR/$hook"
    echo "Installed $hook hook"
  fi
done

echo "Done."
