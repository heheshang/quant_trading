#!/bin/bash
# =============================================================================
# detect-platform.sh — Agent Platform Auto-Detection
# Sourced by init.sh and other Harness scripts at session start.
# Detects which AI coding agent is running and exports platform-specific vars.
#
# Usage: source .harness/scripts/detect-platform.sh
#
# Detection priority:
#   1. Environment variable override (PLATFORM_FORCE=claude|codex|generic)
#   2. Presence of well-known config directories / files
#   3. Available tool binaries (if unambiguous)
#   4. Fallback to generic
#
# Exports:
#   PLATFORM           — detected platform (claude|codex|generic)
#   PLATFORM_CONFIDENCE— detection confidence (high|medium|low)
#   MCP_CONFIG_PATH    — path to MCP server config file
#   FILE_READ_TOOL     — tool name for reading files
#   FILE_WRITE_TOOL    — tool name for writing files
#   SHELL_TOOL         — tool name for shell commands
#   CI_AVAILABLE       — (true|false) whether CI MCP is detected
# =============================================================================

detect_platform() {
  # 0. Allow forced override
  if [ -n "${PLATFORM_FORCE}" ]; then
    case "${PLATFORM_FORCE}" in
      claude|codex|generic)
        export PLATFORM="${PLATFORM_FORCE}"
        export PLATFORM_CONFIDENCE="high"
        ;;
      *)
        echo "  ⚠️  Unknown PLATFORM_FORCE='${PLATFORM_FORCE}'. Fallback to auto-detect."
        ;;
    esac
  fi

  # 1. Claude Code detection
  if [ -z "${PLATFORM}" ] && [ -d "${HOME}/.claude" ]; then
    if [ -f "${HOME}/.claude/settings.json" ] || [ -f "${HOME}/.claude/claude.md" ]; then
      export PLATFORM="claude"
      export PLATFORM_CONFIDENCE="high"
      export MCP_CONFIG_PATH="${HOME}/.claude/settings.json"
    else
      export PLATFORM="claude"
      export PLATFORM_CONFIDENCE="medium"
      export MCP_CONFIG_PATH="${HOME}/.claude/settings.json"
    fi
  fi

  # 2. Codex CLI detection
  if [ -z "${PLATFORM}" ] && [ -d "${HOME}/.codex" ]; then
    if [ -f "${HOME}/.codex/mcp.json" ] || [ -f "${HOME}/.codex/config.json" ]; then
      export PLATFORM="codex"
      export PLATFORM_CONFIDENCE="high"
      export MCP_CONFIG_PATH="${HOME}/.codex/mcp.json"
    else
      export PLATFORM="codex"
      export PLATFORM_CONFIDENCE="medium"
      export MCP_CONFIG_PATH="${HOME}/.codex/mcp.json"
    fi
  fi

  # 3. OpenCode detection
  # Check global opencode config first, then project-level .opencode/ directory
  if [ -z "${PLATFORM}" ]; then
    if [ -f "${HOME}/.config/opencode/opencode.json" ]; then
      export PLATFORM="opencode"
      export PLATFORM_CONFIDENCE="high"
      export MCP_CONFIG_PATH="${HOME}/.config/opencode/opencode.json"
    elif [ -f ".opencode/lsp.json" ] || [ -f "opencode.jsonc" ] || [ -f "opencode.json" ]; then
      export PLATFORM="opencode"
      export PLATFORM_CONFIDENCE="medium"
      export MCP_CONFIG_PATH=".opencode/"
    fi
  fi

  # 4. Environment variable detection (used by CI or containers)
  if [ -z "${PLATFORM}" ]; then
    if [ "${CLAUDE_CODE}" = "1" ] || [ "${CLAUDE_CODE}" = "true" ]; then
      export PLATFORM="claude"
      export PLATFORM_CONFIDENCE="high"
      export MCP_CONFIG_PATH="${HOME}/.claude/settings.json"
    elif [ "${CODEX_CLI}" = "1" ] || [ "${CODEX_CLI}" = "true" ]; then
      export PLATFORM="codex"
      export PLATFORM_CONFIDENCE="high"
      export MCP_CONFIG_PATH="${HOME}/.codex/mcp.json"
    fi
  fi

  # 4. Cursor IDE detection
  if [ -z "${PLATFORM}" ] && [ -n "${CURSOR_TRACE_ID}" ]; then
    export PLATFORM="cursor"
    export PLATFORM_CONFIDENCE="low"
    export MCP_CONFIG_PATH=""
  fi

  # 5. GitHub Actions CI detection
  if [ -z "${PLATFORM}" ] && [ "${GITHUB_ACTIONS}" = "true" ]; then
    export PLATFORM="generic"
    export PLATFORM_CONFIDENCE="low"
    export MCP_CONFIG_PATH=""
    export CI_AVAILABLE="true"
  fi

  # 6. Fallback to generic
  if [ -z "${PLATFORM}" ]; then
    export PLATFORM="generic"
    export PLATFORM_CONFIDENCE="low"
    export MCP_CONFIG_PATH=""
  fi

  # Set default CI_AVAILABLE
  : "${CI_AVAILABLE:=false}"

  # Tool mapping (informational — actual tool names differ per platform)
  case "${PLATFORM}" in
    claude)
      export FILE_READ_TOOL="Read"
      export FILE_WRITE_TOOL="Write / Edit"
      export SHELL_TOOL="Bash"
      ;;
    codex)
      export FILE_READ_TOOL="read"
      export FILE_WRITE_TOOL="write / edit"
      export SHELL_TOOL="bash / shell"
      ;;
    opencode)
      export FILE_READ_TOOL="Read"
      export FILE_WRITE_TOOL="Write / Edit"
      export SHELL_TOOL="Bash / interactive_bash"
      ;;
    cursor)
      export FILE_READ_TOOL="Read"
      export FILE_WRITE_TOOL="Edit"
      export SHELL_TOOL="terminal"
      ;;
    generic)
      export FILE_READ_TOOL="cat / less"
      export FILE_WRITE_TOOL="echo / tee"
      export SHELL_TOOL="sh / bash"
      ;;
  esac
}

# Auto-detect on source
detect_platform

# Print detected configuration
echo "  🖥️  Platform: ${PLATFORM} (confidence: ${PLATFORM_CONFIDENCE})"
echo "  ⚙️  MCP config: ${MCP_CONFIG_PATH:-"(none)"}"
echo "  🔧 CI available: ${CI_AVAILABLE}"
echo "  📖 File read: ${FILE_READ_TOOL}"
echo "  ✏️  File write: ${FILE_WRITE_TOOL}"
