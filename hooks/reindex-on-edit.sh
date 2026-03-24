#!/usr/bin/env bash
# PostToolUse hook: auto-reindex when ontology files are edited
# Called by Claude Code after Edit/Write tool use
#
# Reads tool_input from stdin (JSON), checks if file_path is in an ontology dir,
# and triggers reindex if so.

set -euo pipefail

INPUT=$(cat)

# Extract file path from tool input
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.filePath // empty' 2>/dev/null)

if [ -z "$FILE_PATH" ]; then
    exit 0
fi

# Check if path contains ontology directories
if echo "$FILE_PATH" | grep -qE '(\.claude/ontology/|\.claude/personas/)'; then
    PERSONA_DIR="${HOME}/.claude/personas"
    PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/ontology"
    onto reindex-if-path "$FILE_PATH" --persona-dir "$PERSONA_DIR" --project-dir "$PROJECT_DIR" 2>/dev/null || true
fi
