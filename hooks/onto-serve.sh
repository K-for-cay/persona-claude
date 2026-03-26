#!/usr/bin/env bash
# Wrapper for onto serve that resolves persona directory based on installation mode
#
# Global install → ~/.claude/personas/
# Local install  → derived from CLAUDE_PLUGIN_ROOT
# ONTO_PERSONA_DIR env override takes precedence.

set -euo pipefail

if [ -z "${ONTO_PERSONA_DIR:-}" ]; then
    local_plugin_root="${CLAUDE_PLUGIN_ROOT:-}"
    if [ -n "$local_plugin_root" ]; then
        if echo "$local_plugin_root" | grep -q "^${HOME}/\.claude/"; then
            export ONTO_PERSONA_DIR="${HOME}/.claude/personas"
        else
            claude_dir=$(echo "$local_plugin_root" | sed 's|/\.claude/plugins/.*|/.claude|')
            export ONTO_PERSONA_DIR="${claude_dir}/personas"
        fi
    else
        export ONTO_PERSONA_DIR="${HOME}/.claude/personas"
    fi
fi

exec onto serve "$@"
