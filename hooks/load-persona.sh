#!/usr/bin/env bash
# SessionStart hook: load persona ontology at session start
# Outputs persona _index.md and core identity nodes to stdout,
# which Claude Code injects into context automatically.

set -euo pipefail

# Determine persona directory:
# 1. If ONTO_PERSONA_DIR is set, use it
# 2. If .claude/personas/ exists in current project, use local
# 3. Fall back to ~/.claude/personas/ (global)
if [ -n "${ONTO_PERSONA_DIR:-}" ]; then
    PERSONA_DIR="$ONTO_PERSONA_DIR"
elif [ -d "${CLAUDE_PROJECT_DIR:-.}/.claude/personas" ]; then
    PERSONA_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/personas"
elif [ -d "${HOME}/.claude/personas" ]; then
    PERSONA_DIR="${HOME}/.claude/personas"
else
    exit 0
fi

# Load _index.md (entry point)
if [ -f "$PERSONA_DIR/_index.md" ]; then
    echo "--- Persona Ontology (from $PERSONA_DIR) ---"
    cat "$PERSONA_DIR/_index.md"
    echo ""
fi

# Load all identity nodes (core self)
if [ -d "$PERSONA_DIR/identity" ]; then
    for f in "$PERSONA_DIR/identity"/*.md; do
        [ -f "$f" ] || continue
        cat "$f"
        echo ""
    done
fi

# Load all style nodes (communication patterns)
if [ -d "$PERSONA_DIR/style" ]; then
    for f in "$PERSONA_DIR/style"/*.md; do
        [ -f "$f" ] || continue
        cat "$f"
        echo ""
    done
fi
