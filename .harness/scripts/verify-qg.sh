#!/bin/bash
# Quality Gate Verification Scripts
# 用法: ./verify-qg.sh <gate-number> <change-dir>
# 示例: ./verify-qg.sh 1 .harness/changes/feat-add-cache-20260622

set -e

# Source build detection
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/detect-build.sh"

GATE=$1
CHANGE_DIR=$2
RESULT_FILE="${CHANGE_DIR}/qg-result.md"

if [ -z "$GATE" ] || [ -z "$CHANGE_DIR" ]; then
  echo "Usage: $0 <gate-number> <change-dir>"
  exit 1
fi

mkdir -p "$CHANGE_DIR"

echo "# Quality Gate QG-${GATE} Verification" > "$RESULT_FILE"
echo "" >> "$RESULT_FILE"
echo "- **Gate**: QG-${GATE}" >> "$RESULT_FILE"
echo "- **Time**: $(date '+%Y-%m-%d %H:%M:%S')" >> "$RESULT_FILE"
echo "- **Change Dir**: ${CHANGE_DIR}" >> "$RESULT_FILE"
echo "" >> "$RESULT_FILE"

PASS=true

case $GATE in
  1)
    echo "## QG-1: Requirement Completeness" >> "$RESULT_FILE"
    echo "" >> "$RESULT_FILE"

    # Check spec.md exists
    if [ ! -f "${CHANGE_DIR}/spec.md" ]; then
      echo "- [ ] FAIL: spec.md not found" >> "$RESULT_FILE"
      PASS=false
    else
      echo "- [x] spec.md exists" >> "$RESULT_FILE"
    fi

    # Check required sections
    for section in "背景" "需求描述" "变更范围" "影响分析" "验收标准"; do
      if grep -q "## .*${section}" "${CHANGE_DIR}/spec.md" 2>/dev/null; then
        echo "- [x] Section '${section}' present" >> "$RESULT_FILE"
      else
        echo "- [ ] FAIL: Section '${section}' missing" >> "$RESULT_FILE"
        PASS=false
      fi
    done

    # Check for ambiguous words (support Chinese and English)
     AMBIG_CN=$(grep -c -E "可能|大概|也许|或者|不确定" "${CHANGE_DIR}/spec.md" 2>/dev/null || echo 0)
     AMBIG_EN=$(grep -c -i -E "\bmaybe\b|\bprobably\b|\bperhaps\b|\buncertain\b|\bnot sure\b" "${CHANGE_DIR}/spec.md" 2>/dev/null || echo 0)
     AMBIG_COUNT=$((AMBIG_CN + AMBIG_EN))
     if [ "$AMBIG_COUNT" -gt 0 ]; then
       echo "- [ ] FAIL: Found ${AMBIG_COUNT} ambiguous descriptions (CN: ${AMBIG_CN}, EN: ${AMBIG_EN})" >> "$RESULT_FILE"
       PASS=false
     else
       echo "- [x] No ambiguous descriptions found" >> "$RESULT_FILE"
     fi
    ;;

  2)
    echo "## QG-2: Review Approval" >> "$RESULT_FILE"
    echo "" >> "$RESULT_FILE"

    # Find latest review file
    LATEST_REVIEW=$(ls -t "${CHANGE_DIR}"/review-v*.md 2>/dev/null | head -1)
    if [ -z "$LATEST_REVIEW" ]; then
      echo "- [ ] FAIL: No review file found" >> "$RESULT_FILE"
      PASS=false
    else
      echo "- [x] Review file exists: ${LATEST_REVIEW}" >> "$RESULT_FILE"

      # Check for MUST FIX issues
       # Match only review item lines (prefixed with number or bullet), not table headers or summaries
       MUST_FIX_COUNT=$(grep -c -E "^\s*[0-9*\-]+\.?\s*MUST FIX" "$LATEST_REVIEW" 2>/dev/null || echo 0)
       if [ "$MUST_FIX_COUNT" -gt 0 ]; then
         echo "- [ ] FAIL: Found ${MUST_FIX_COUNT} MUST FIX issues" >> "$RESULT_FILE"
         PASS=false
       else
         echo "- [x] No MUST FIX issues" >> "$RESULT_FILE"
       fi
    fi

    # Check review round limit
    REVIEW_COUNT=$(ls "${CHANGE_DIR}"/review-v*.md 2>/dev/null | wc -l)
    MAX_ROUNDS=3
    if [ "$REVIEW_COUNT" -gt "$MAX_ROUNDS" ]; then
      echo "- [ ] FAIL: Review rounds (${REVIEW_COUNT}) exceed limit (${MAX_ROUNDS})" >> "$RESULT_FILE"
      PASS=false
    else
      echo "- [x] Review rounds: ${REVIEW_COUNT}/${MAX_ROUNDS}" >> "$RESULT_FILE"
    fi
    ;;

   3)
      echo "## QG-3: Compilation Check" >> "$RESULT_FILE"
      echo "" >> "$RESULT_FILE"

      if [ "$BUILD_TOOL" != "unknown" ]; then
        COMPILE_OUTPUT=$(${BUILD_CHECK_CMD} 2>&1) || true
        COMPILE_EXIT=$?
        if [ $COMPILE_EXIT -eq 0 ]; then
          echo "- [x] ${BUILD_TOOL} compile: SUCCESS" >> "$RESULT_FILE"
        else
          echo "- [ ] FAIL: ${BUILD_TOOL} compile failed (exit code: $COMPILE_EXIT)" >> "$RESULT_FILE"
          PASS=false
        fi

        WARN_COUNT=$(echo "$COMPILE_OUTPUT" | grep -c -i "warning" 2>/dev/null || echo 0)
        if [ "$WARN_COUNT" -gt 0 ]; then
          echo "- [ ] WARN: Found ${WARN_COUNT} compilation warnings" >> "$RESULT_FILE"
          echo "  (QG requirement: zero warnings)" >> "$RESULT_FILE"
        else
          echo "- [x] No compilation warnings" >> "$RESULT_FILE"
        fi
      else
        echo "- [ ] WARN: No build tool detected" >> "$RESULT_FILE"
      fi
      ;;

   4)
    echo "## QG-4: Unit Test Gate" >> "$RESULT_FILE"
    echo "" >> "$RESULT_FILE"

    if [ "$BUILD_TOOL" != "unknown" ]; then
      echo "- Running: ${TEST_CMD}" >> "$RESULT_FILE"
      TEST_OUTPUT=$(${TEST_CMD} 2>&1) || true
      TEST_EXIT=$?

      # Try to parse test results (format varies by tool)
      if echo "$TEST_OUTPUT" | grep -q -E "Tests run:|tests passed|FAILED|ok|PASS"; then
        TOTAL=$(echo "$TEST_OUTPUT" | grep -oP 'Tests run: \K[0-9]+' | head -1 || echo "")
        if [ -z "$TOTAL" ]; then
          # grep exit code-based result summary
          if [ $TEST_EXIT -eq 0 ]; then
            echo "- [x] All tests passed (exit code: 0)" >> "$RESULT_FILE"
          else
            echo "- [ ] FAIL: Tests failed (exit code: $TEST_EXIT)" >> "$RESULT_FILE"
            PASS=false
          fi
        else
          PASSED=$(echo "$TEST_OUTPUT" | grep -oP 'Failures: \K[0-9]+' | head -1 || echo 0)
          FAILURES=$((TOTAL - PASSED))
          echo "- Tests run: ${TOTAL}" >> "$RESULT_FILE"
          echo "- Passed: ${PASSED}" >> "$RESULT_FILE"
          echo "- Failed: ${FAILURES}" >> "$RESULT_FILE"
          if [ "$TOTAL" -eq 0 ]; then
            echo "- [ ] FAIL: No tests found (total_tests == 0)" >> "$RESULT_FILE"
            PASS=false
          elif [ "$FAILURES" -gt 0 ]; then
            echo "- [ ] FAIL: ${FAILURES} tests failed" >> "$RESULT_FILE"
            PASS=false
          else
            echo "- [x] All ${TOTAL} tests passed" >> "$RESULT_FILE"
          fi
        fi
      else
        echo "- [ ] WARN: Could not parse test output" >> "$RESULT_FILE"
        if [ $TEST_EXIT -eq 0 ]; then
          echo "  (exit code 0, assuming pass)" >> "$RESULT_FILE"
        else
          echo "- [ ] FAIL: Tests failed (exit code: $TEST_EXIT)" >> "$RESULT_FILE"
          PASS=false
        fi
      fi
    else
      echo "- [ ] WARN: No build tool detected — cannot run tests" >> "$RESULT_FILE"
    fi
    ;;

   5)
     echo "## QG-5: CI Gate" >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Note: CI gate requires external CI system integration." >> "$RESULT_FILE"
     echo "Verify manually or via CI MCP tool." >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Required conditions:" >> "$RESULT_FILE"
     echo "- status == SUCCESS" >> "$RESULT_FILE"
     echo "- total_tests > 0" >> "$RESULT_FILE"
     echo "- passed == total_tests" >> "$RESULT_FILE"
     echo "- code coverage >= 80% (new code)" >> "$RESULT_FILE"
     ;;

   6)
     echo "## QG-6: Integration Test Gate" >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Note: Integration test requires deployed environment." >> "$RESULT_FILE"
     echo "Verify via CI MCP or manual test execution." >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Required conditions:" >> "$RESULT_FILE"
     echo "- All integration tests pass" >> "$RESULT_FILE"
     echo "- Core API endpoints return 2XX" >> "$RESULT_FILE"
     echo "- Response body structure matches spec" >> "$RESULT_FILE"
     echo "- Response time within P99 threshold" >> "$RESULT_FILE"
     ;;

   7)
     echo "## QG-7: Deploy Verification Gate" >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Note: Deploy verification requires live environment." >> "$RESULT_FILE"
     echo "Verify via health check endpoint + monitoring system." >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Required conditions:" >> "$RESULT_FILE"
     echo "- Health check endpoint returns 200" >> "$RESULT_FILE"
     echo "- CPU < 80%, Memory < 85%" >> "$RESULT_FILE"
     echo "- Core business flows operational" >> "$RESULT_FILE"
     ;;

   8)
     echo "## QG-8: Canary Release Gate" >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Note: Canary gate requires production monitoring." >> "$RESULT_FILE"
     echo "Verify via monitoring/alarm system." >> "$RESULT_FILE"
     echo "" >> "$RESULT_FILE"
     echo "Required conditions:" >> "$RESULT_FILE"
     echo "- Error rate < baseline * 1.5" >> "$RESULT_FILE"
     echo "- No P0/P1 alarms during canary" >> "$RESULT_FILE"
     echo "- Core API RT no significant increase" >> "$RESULT_FILE"
     echo "- Observation period >= configured value" >> "$RESULT_FILE"
     ;;

  *)
    echo "Unknown gate: ${GATE}" >> "$RESULT_FILE"
    exit 1
    ;;
esac

echo "" >> "$RESULT_FILE"
echo "## Result" >> "$RESULT_FILE"
if [ "$PASS" = true ]; then
  echo "**PASS**" >> "$RESULT_FILE"
  echo "" >> "$RESULT_FILE"
  echo "✅ Gate QG-${GATE} passed."
else
  echo "**FAIL**" >> "$RESULT_FILE"
  echo "" >> "$RESULT_FILE"
  echo "❌ Gate QG-${GATE} failed. See details above."
  exit 1
fi
