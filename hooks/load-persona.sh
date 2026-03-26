#!/usr/bin/env bash
# SessionStart hook: load persona ontology at session start
# Outputs persona _index.md and core identity nodes to stdout,
# which Claude Code injects into context automatically.
#
# Installation mode determines persona directory:
#   Global install → ~/.claude/personas/
#   Local install  → ${CLAUDE_PROJECT_DIR}/.claude/personas/
# ONTO_PERSONA_DIR env override takes precedence.

set -euo pipefail

# Resolve persona directory based on installation mode
resolve_persona_dir() {
    # Explicit override
    if [ -n "${ONTO_PERSONA_DIR:-}" ]; then
        echo "$ONTO_PERSONA_DIR"
        return
    fi

    # Detect installation mode from plugin root path
    # Global: ~/.claude/plugins/... → persona at ~/.claude/personas/
    # Local:  <repo>/.claude/plugins/... → persona at <repo>/.claude/personas/
    local plugin_root="${CLAUDE_PLUGIN_ROOT:-}"
    if [ -n "$plugin_root" ]; then
        # Check if plugin is under ~/.claude/ (global)
        if echo "$plugin_root" | grep -q "^${HOME}/\.claude/"; then
            echo "${HOME}/.claude/personas"
            return
        fi
        # Local install: derive project root from plugin path
        # <project>/.claude/plugins/... → <project>/.claude/personas/
        local claude_dir
        claude_dir=$(echo "$plugin_root" | sed 's|/\.claude/plugins/.*|/.claude|')
        echo "${claude_dir}/personas"
        return
    fi

    # Fallback: use CLAUDE_PROJECT_DIR if set, otherwise home
    if [ -n "${CLAUDE_PROJECT_DIR:-}" ]; then
        echo "${CLAUDE_PROJECT_DIR}/.claude/personas"
    else
        echo "${HOME}/.claude/personas"
    fi
}

PERSONA_DIR=$(resolve_persona_dir)

# Exit silently if persona directory doesn't exist
[ -d "$PERSONA_DIR" ] || exit 0

# Load _index.md (entry point)
if [ -f "$PERSONA_DIR/_index.md" ]; then
    echo "--- Persona Ontology ---"
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
