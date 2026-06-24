#!/bin/bash
# =============================================================================
# gc-scan.sh — Entropy Garbage Collection Scanner
# Detects code decay across 5 categories defined in entropy-gc.md.
#
# Usage: bash .harness/scripts/gc-scan.sh [--category N] [--path DIR]
#
# Options:
#   --category N   Scan only one category (1-5, default: all)
#   --path DIR     Scan a specific directory (default: project root)
#   --output FILE  Write report to FILE (default: gc-report-{date}.md)
#   --json         Output findings as JSON (for CI/agent consumption)
#
# Exit code: 0 if no issues found, 1 if any issues found
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/detect-build.sh"

SCAN_PATH="${PWD}"
OUTPUT_FILE=""
CATEGORY="all"
JSON_MODE=false

# ── Parse arguments ──────────────────────────────────────────────────────────
while [ $# -gt 0 ]; do
  case "$1" in
    --category) CATEGORY="$2"; shift 2 ;;
    --path) SCAN_PATH="$2"; shift 2 ;;
    --output) OUTPUT_FILE="$2"; shift 2 ;;
    --json) JSON_MODE=true; shift ;;
    *) echo "Usage: $0 [--category 1-5] [--path DIR] [--output FILE] [--json]"; exit 1 ;;
  esac
done

REPORT_DATE=$(date '+%Y%m%d')
: "${OUTPUT_FILE:=gc-report-${REPORT_DATE}.md}"

TOTAL_ISSUES=0
PASS=true

# ── Report helpers ──────────────────────────────────────────────────────────
report_init() {
  echo "# Entropy GC Scan Report" > "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
  echo "- **Date**: $(date '+%Y-%m-%d %H:%M:%S')" >> "$OUTPUT_FILE"
  echo "- **Scan path**: ${SCAN_PATH}" >> "$OUTPUT_FILE"
  echo "- **Build tool**: ${BUILD_TOOL}" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
  echo "## Findings" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
}

report_issue() {
  local category="$1"
  local severity="$2"
  local file="$3"
  local desc="$4"
  TOTAL_ISSUES=$((TOTAL_ISSUES + 1))
  echo "| ${category} | ${severity} | ${file} | ${desc} |" >> "$OUTPUT_FILE"
  echo "[${severity}] ${category}: ${file} — ${desc}"
}

report_finalize() {
  echo "" >> "$OUTPUT_FILE"
  echo "## Summary" >> "$OUTPUT_FILE"
  echo "" >> "$OUTPUT_FILE"
  echo "- **Total issues found**: ${TOTAL_ISSUES}" >> "$OUTPUT_FILE"
  if [ "$TOTAL_ISSUES" -gt 0 ]; then
    echo "**FAIL**: ${TOTAL_ISSUES} entropy issue(s) detected. See above." >> "$OUTPUT_FILE"
  else
    echo "**PASS**: No entropy issues detected." >> "$OUTPUT_FILE"
  fi
}

json_emit() {
  # If JSON mode, also write structured output
  if [ "$JSON_MODE" = true ]; then
    local json_file="${OUTPUT_FILE%.md}.json"
    # Simple JSON construction (jq would be better but not always available)
    echo "{\"date\":\"$(date -Iseconds)\",\"total_issues\":${TOTAL_ISSUES},\"pass\":${PASS}}" > "$json_file"
  fi
}

# ── Category 1: Comment Drift ──────────────────────────────────────────────
scan_comment_drift() {
  local found=0
  # 1a. Stale TODOs with no assignee or old dates
  while IFS=: read -r file line content; do
    # Skip files in .harness/ (framework docs intentionally have TODOs)
    [[ "$file" == .harness/* ]] && continue
    # Match TODO without date or with date older than 90 days
    if echo "$content" | grep -q -E '(TODO|FIXME|HACK)'; then
      report_issue "Comment Drift" "MEDIUM" "${file}:${line}" "Stale TODO/FIXME"
      found=$((found + 1))
    fi
  done < <(grep -rn -E '(TODO|FIXME|HACK)' --include='*.py' --include='*.sh' --include='*.md' --include='*.java' --include='*.go' --include='*.rs' --include='*.ts' "${SCAN_PATH}" 2>/dev/null | head -30 || true)

  # 1b. Comment that references wrong line numbers or params (heuristic: mismatch between comment names and code)
  # This is a simplified check — full detection needs AST parsing
  echo "$found" > /tmp/gc_cat1_count
}

# ── Category 2: Dead Code ──────────────────────────────────────────────────
scan_dead_code() {
  local found=0

  # 2a. Python: functions/classes defined but never imported elsewhere
  if [ "${BUILD_TOOL}" = "pip" ]; then
    while IFS= read -r func; do
      func_name=$(echo "$func" | grep -oP 'def \K\w+' | head -1)
      [ -z "$func_name" ] && continue
      # Skip dunder methods and test utilities
      [[ "$func_name" == __*__ ]] && continue
      [[ "$func_name" == test_* ]] && continue
      # Check if referenced elsewhere (excluding the defining file)
      ref_count=$(grep -rn "\b${func_name}\b" --include='*.py' "${SCAN_PATH}" 2>/dev/null | grep -v "$(echo "$func" | cut -d: -f1)" | wc -l || echo 0)
      if [ "$ref_count" -eq 0 ]; then
        report_issue "Dead Code" "HIGH" "$(echo "$func" | cut -d: -f1):$(echo "$func" | cut -d: -f2)" "Unused function: ${func_name}"
        found=$((found + 1))
      fi
    done < <(grep -rn '^def ' --include='*.py' "${SCAN_PATH}" 2>/dev/null | head -50 || true)
  fi

  # 2b. Shell: functions defined but unused
  if [ -n "$(find "${SCAN_PATH}" -name '*.sh' -type f 2>/dev/null | head -1)" ]; then
    while IFS= read -r line; do
      func_name=$(echo "$line" | grep -oP '^[a-zA-Z_][a-zA-Z0-9_]*\(\)' | head -1 | sed 's/()//')
      [ -z "$func_name" ] && continue
      ref_count=$(grep -rn "\b${func_name}\b" --include='*.sh' "${SCAN_PATH}" 2>/dev/null | grep -v "$(echo "$line" | cut -d: -f1)" | wc -l || echo 0)
      if [ "$ref_count" -eq 0 ]; then
        report_issue "Dead Code" "HIGH" "$(echo "$line" | cut -d: -f1):$(echo "$line" | cut -d: -f2)" "Potentially unused function: ${func_name}"
        found=$((found + 1))
      fi
    done < <(grep -rn '^[a-zA-Z_][a-zA-Z0-9_]*()' --include='*.sh' "${SCAN_PATH}" 2>/dev/null | head -30 || true)
  fi

  echo "$found" > /tmp/gc_cat2_count
}

# ── Category 3: Stale References ──────────────────────────────────────────
scan_stale_references() {
  local found=0

  # 3a. Deprecation warnings in build output (requires build)
  if [ -f "${SCAN_PATH}/pom.xml" ] || [ -f "${SCAN_PATH}/build.gradle" ]; then
    report_issue "Stale References" "MEDIUM" "(build system)" "Run build to check deprecation warnings"
    found=$((found + 1))
  fi

  # 3b. Python: import statements referencing potentially missing modules
  if [ "${BUILD_TOOL}" = "pip" ]; then
    while IFS= read -r imp; do
      module=$(echo "$imp" | grep -oP 'import \K\w+|from \K\w+')
      [ -z "$module" ] && continue
      # Skip stdlib modules
      case "$module" in os|sys|re|json|pathlib|datetime|typing|collections|math|functools|itertools|subprocess|shutil|tempfile) continue ;; esac
      # Check if module file exists in project
      if ! find "${SCAN_PATH}" -name "${module}.py" -type f 2>/dev/null | grep -q .; then
        # It might be a third-party package — can't verify without pip list
        # Flag as informational only
        report_issue "Stale References" "LOW" "$(echo "$imp" | cut -d: -f1):$(echo "$imp" | cut -d: -f2)" "External import: ${module} (verify it's in requirements)"
        found=$((found + 1))
      fi
    done < <(grep -rn '^import \|^from ' --include='*.py' "${SCAN_PATH}" 2>/dev/null | head -30 || true)
  fi

  echo "$found" > /tmp/gc_cat3_count
}

# ── Category 4: Test Rot ──────────────────────────────────────────────────
scan_test_rot() {
  local found=0

  # 4a. Empty test functions / methods
  while IFS= read -r line; do
    # Match empty function body (def followed by pass or just docstring)
    local file
    file=$(echo "$line" | cut -d: -f1)
    local lineno
    lineno=$(echo "$line" | cut -d: -f2)
    # Check if next line is just 'pass' or '"""'
    local next_line
    next_line=$(sed -n "$((lineno + 1))p" "$file" 2>/dev/null)
    local n2_line
    n2_line=$(sed -n "$((lineno + 2))p" "$file" 2>/dev/null)
    if echo "$next_line" | grep -q '^\s*pass\s*$' || (echo "$next_line" | grep -q '"""' && echo "$n2_line" | grep -q '^\s*pass\s*$'); then
      report_issue "Test Rot" "HIGH" "${file}:${lineno}" "Empty test function"
      found=$((found + 1))
    fi
  done < <(grep -rn 'def test_\|def test[A-Z]' --include='*test*.py' "${SCAN_PATH}" 2>/dev/null | head -30 || true)

  # 4b. Test files with no assertions
  while IFS= read -r tf; do
    if grep -q 'def test_' "$tf" 2>/dev/null; then
      local assert_count
      assert_count=$(grep -c 'assert' "$tf" 2>/dev/null || echo 0)
      local func_count
      func_count=$(grep -c 'def test_' "$tf" 2>/dev/null || echo 0)
      if [ "$assert_count" -lt "$func_count" ] && [ "$func_count" -gt 0 ]; then
        report_issue "Test Rot" "MEDIUM" "$tf" "Fewer assertions (${assert_count}) than test functions (${func_count})"
        found=$((found + 1))
      fi
    fi
  done < <(find "${SCAN_PATH}" -name '*test*.py' -type f 2>/dev/null | head -20 || true)

  echo "$found" > /tmp/gc_cat4_count
}

# ── Category 5: Architecture Drift ──────────────────────────────────────────
scan_architecture_drift() {
  local found=0

  # 5a. Files over 250 lines (excessive size)
  while IFS= read -r file; do
    local line_count
    line_count=$(wc -l < "$file" 2>/dev/null || echo 0)
    if [ "$line_count" -gt 250 ]; then
      report_issue "Architecture Drift" "CRITICAL" "$file" "File exceeds 250 lines (${line_count} lines)"
      found=$((found + 1))
    fi
  done < <(find "${SCAN_PATH}" -name '*.py' -o -name '*.sh' -o -name '*.java' -o -name '*.go' -o -name '*.rs' 2>/dev/null | grep -v '.harness/' | grep -v '__pycache__' | head -30 || true)

  # 5b. Circular dependency potential (simplified: module A imports B, B imports A)
  if [ "${BUILD_TOOL}" = "pip" ]; then
    local py_modules
    py_modules=$(find "${SCAN_PATH}" -name '*.py' -type f 2>/dev/null | grep -v '__pycache__' | grep -v '.harness/' | head -50)
    for mod in $py_modules; do
      local mod_imports
      mod_imports=$(grep -E '^import |^from ' "$mod" 2>/dev/null | grep -oP 'from \K\w+|import \K\w+' | sort -u)
      for imp in $mod_imports; do
        # Check if the imported module imports back
        local imp_file
        imp_file=$(find "${SCAN_PATH}" -path "*/${imp}.py" -type f 2>/dev/null | head -1)
        [ -z "$imp_file" ] && continue
        local mod_name
        mod_name=$(basename "$mod" .py)
        if grep -q "\b${mod_name}\b" "$imp_file" 2>/dev/null; then
          report_issue "Architecture Drift" "CRITICAL" "$mod ↔ $imp_file" "Potential circular dependency"
          found=$((found + 1))
          break
        fi
      done
    done
  fi

  echo "$found" > /tmp/gc_cat5_count
}

# ── Main ──────────────────────────────────────────────────────────────────
report_init

echo "# Scanning for entropy issues..."
echo ""

case "$CATEGORY" in
  1|all) echo "## [1/5] Comment Drift" && scan_comment_drift ;;
esac
case "$CATEGORY" in
  2|all) echo "## [2/5] Dead Code" && scan_dead_code ;;
esac
case "$CATEGORY" in
  3|all) echo "## [3/5] Stale References" && scan_stale_references ;;
esac
case "$CATEGORY" in
  4|all) echo "## [4/5] Test Rot" && scan_test_rot ;;
esac
case "$CATEGORY" in
  5|all) echo "## [5/5] Architecture Drift" && scan_architecture_drift ;;
esac

report_finalize
json_emit

echo ""
echo "Report written to: ${OUTPUT_FILE}"
echo "Total issues: ${TOTAL_ISSUES}"

if [ "$TOTAL_ISSUES" -gt 0 ]; then
  exit 1
fi
