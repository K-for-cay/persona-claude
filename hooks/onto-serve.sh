#!/usr/bin/env bash
# Wrapper for onto serve that resolves persona directory with local-first fallback
# Priority: ONTO_PERSONA_DIR env > local .claude/personas/ > global ~/.claude/personas/

set -euo pipefail

if [ -z "${ONTO_PERSONA_DIR:-}" ]; then
    LOCAL_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/personas"
    GLOBAL_DIR="${HOME}/.claude/personas"

    if [ -d "$LOCAL_DIR" ]; then
        export ONTO_PERSONA_DIR="$LOCAL_DIR"
    elif [ -d "$GLOBAL_DIR" ]; then
        export ONTO_PERSONA_DIR="$GLOBAL_DIR"
    fi
fi

exec onto serve "$@"
