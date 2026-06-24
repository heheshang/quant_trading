#!/bin/bash
# =============================================================================
# detect-build.sh — Build Tool Auto-Detection
# Sourced by init.sh, verify-qg.sh, and other Harness scripts.
# Detects the project's build tool and exports standardized variables.
#
# Usage: source .harness/scripts/detect-build.sh
#
# Exports:
#   BUILD_TOOL      — detected tool name (maven|gradle|pip|npm|cargo|go|unknown)
#   BUILD_CMD       — command to compile/build
#   BUILD_CHECK_CMD — command to verify the project builds (quiet)
#   TEST_CMD        — command to run unit tests
#   TEST_DIRS       — glob-able test directory pattern(s)
#   LINT_CMD        — command to run linter (if available)
# =============================================================================

detect_build_tool() {
  # Priority order: build file presence

  # Maven
  if [ -f "pom.xml" ]; then
    export BUILD_TOOL="maven"
    export BUILD_CMD="mvn compile"
    export BUILD_CHECK_CMD="mvn compile -q"
    export TEST_CMD="mvn test"
    export TEST_DIRS="src/test/**/*Test.java src/test/**/*Test.ts"
    export LINT_CMD="mvn checkstyle:check 2>/dev/null || echo 'checkstyle not configured'"
    return 0
  fi

  # Gradle
  if [ -f "build.gradle" ] || [ -f "build.gradle.kts" ] || [ -f "gradlew" ]; then
    local gradle_cmd="./gradlew"
    [ ! -x "$gradle_cmd" ] && gradle_cmd="gradle"
    export BUILD_TOOL="gradle"
    export BUILD_CMD="${gradle_cmd} build"
    export BUILD_CHECK_CMD="${gradle_cmd} compileJava -q"
    export TEST_CMD="${gradle_cmd} test"
    export TEST_DIRS="src/test/**/*Test.groovy src/test/**/*Test.java"
    export LINT_CMD="${gradle_cmd} check 2>/dev/null || true"
    return 0
  fi

  # Python (pip/uv)
  if [ -f "pyproject.toml" ] || [ -f "setup.py" ] || [ -f "setup.cfg" ] || [ -f "requirements.txt" ]; then
    export BUILD_TOOL="pip"
    export BUILD_CMD="python -m compileall . -q"
    export BUILD_CHECK_CMD="python -m compileall . -q"
    export TEST_CMD="python -m pytest -q"
    export TEST_DIRS="**/test_*.py **/*_test.py"
    if [ -f "pyproject.toml" ] && grep -q "ruff" pyproject.toml 2>/dev/null; then
      export LINT_CMD="ruff check ."
    elif command -v ruff &>/dev/null; then
      export LINT_CMD="ruff check ."
    elif command -v pylint &>/dev/null; then
      export LINT_CMD="pylint $(git ls-files '*.py' 2>/dev/null || echo '.')"
    else
      export LINT_CMD="python -m flake8 2>/dev/null || echo 'no linter configured'"
    fi
    return 0
  fi

  # Rust / Cargo (priority over npm — Tauri/Rust projects have both Cargo.toml + package.json)
  if [ -f "Cargo.toml" ]; then
    export BUILD_TOOL="cargo"
    export BUILD_CMD="cargo build"
    export BUILD_CHECK_CMD="cargo check -q"
    export TEST_CMD="cargo test"
    export TEST_DIRS="**/*_test.rs"
    export LINT_CMD="cargo clippy -q 2>/dev/null || echo 'clippy not configured'"
    return 0
  fi

  # Node.js / npm
  if [ -f "package.json" ]; then
    export BUILD_TOOL="npm"
    export BUILD_CMD="npm run build"
    export BUILD_CHECK_CMD="npm run build 2>/dev/null || npx tsc --noEmit 2>/dev/null || echo 'no build script'"
    export TEST_CMD="npm test"
    export TEST_DIRS="**/*.test.ts **/*.test.js **/*.spec.ts"
    export LINT_CMD="npx eslint . 2>/dev/null || echo 'no linter configured'"
    return 0
  fi

  # Go
  if [ -f "go.mod" ]; then
    export BUILD_TOOL="go"
    export BUILD_CMD="go build ./..."
    export BUILD_CHECK_CMD="go build ./..."
    export TEST_CMD="go test ./..."
    export TEST_DIRS="**/*_test.go"
    export LINT_CMD="golangci-lint run 2>/dev/null || echo 'no linter configured'"
    return 0
  fi

  # Unknown / generic
  export BUILD_TOOL="unknown"
  export BUILD_CMD="echo 'no build tool detected'"
  export BUILD_CHECK_CMD="echo 'no build tool detected'"
  export TEST_CMD="echo 'no test tool detected'"
  export TEST_DIRS=""
  export LINT_CMD="echo 'no linter detected'"
  return 0     # ← 始终返回 0；结果通过 BUILD_TOOL 变量传递
}

# Auto-detect on source
detect_build_tool

# Print detected configuration
echo "  🔧 Build tool: ${BUILD_TOOL}"
echo "  🔨 Build cmd:  ${BUILD_CMD}"
echo "  🧪 Test cmd:   ${TEST_CMD}"
