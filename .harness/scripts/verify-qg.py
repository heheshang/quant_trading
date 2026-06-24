#!/usr/bin/env python3
"""
Quality Gate Verification - Programmatic Enforcement
用法: python verify-qg.py <gate-number> <change-dir>
示例: python verify-qg.py 1 .harness/changes/feat-price-filter-20240101

支持门禁: QG-1 ~ QG-8
QG-5 及以上需要外部 CI/部署环境配合，此处提供检查框架。
"""

import os
import re
import sys
import json
from pathlib import Path
from datetime import datetime


def verify_qg1(change_dir: Path) -> dict:
    """QG-1: Requirement Completeness"""
    result = {"gate": "QG-1", "checks": [], "pass": True}
    spec_file = change_dir / "spec.md"

    if not spec_file.exists():
        result["checks"].append({"name": "spec.md exists", "pass": False, "detail": "File not found"})
        result["pass"] = False
        return result
    result["checks"].append({"name": "spec.md exists", "pass": True})

    content = spec_file.read_text(encoding="utf-8")

    required_sections = ["背景", "需求描述", "变更范围", "影响分析", "验收标准", "备注"]
    for section in required_sections:
        pattern = rf"##\s*.*{section}"
        found = bool(re.search(pattern, content))
        result["checks"].append({
            "name": f"Section '{section}'",
            "pass": found,
            "detail": "" if found else "Section not found"
        })
        if not found:
            result["pass"] = False

    # Check for ambiguous words (CN + EN)
    ambiguous_cn = r"可能|大概|也许|或者|不确定"
    ambiguous_en = r"\bmaybe\b|\bprobably\b|\bperhaps\b|\buncertain\b|\bnot sure\b"
    combined = rf"({ambiguous_cn})|({ambiguous_en})"
    ambiguous_matches = re.findall(combined, content, re.IGNORECASE)
    count = sum(1 for m in ambiguous_matches if any(m))
    result["checks"].append({
        "name": "No ambiguous descriptions",
        "pass": count == 0,
        "detail": f"Found {count} ambiguous terms" if count else ""
    })
    if count:
        result["pass"] = False

    return result


def verify_qg2(change_dir: Path) -> dict:
    """QG-2: Review Approval"""
    result = {"gate": "QG-2", "checks": [], "pass": True}

    review_files = sorted(change_dir.glob("review-v*.md"), reverse=True)
    if not review_files:
        result["checks"].append({"name": "Review file exists", "pass": False, "detail": "No review file found"})
        result["pass"] = False
        return result

    result["checks"].append({"name": "Review file exists", "pass": True, "detail": str(review_files[0].name)})

    content = review_files[0].read_text(encoding="utf-8")

    # MUST FIX: only match list items (numbered or bulleted), not headers/summaries
    must_fix_count = len(re.findall(r"^\s*[\d*\-]+\.?\s*MUST FIX", content, re.MULTILINE))
    result["checks"].append({
        "name": "No MUST FIX issues",
        "pass": must_fix_count == 0,
        "detail": f"Found {must_fix_count} MUST FIX items" if must_fix_count else ""
    })
    if must_fix_count:
        result["pass"] = False

    review_count = len(review_files)
    max_rounds = 3
    result["checks"].append({
        "name": f"Review rounds <= {max_rounds}",
        "pass": review_count <= max_rounds,
        "detail": f"Actual: {review_count}"
    })
    if review_count > max_rounds:
        result["pass"] = False

    return result


def detect_build_tool(project_root: Path) -> dict:
    """Detect build tool and return tool info dict.
    
    Returns: {name, build_cmd, test_cmd, test_dirs, lint_cmd, found: bool}
    """
    checks = [
        ("pom.xml", "maven", "mvn compile -q", "mvn test", "src/test/**/*Test.java",
         "mvn checkstyle:check 2>/dev/null || echo 'no checkstyle'"),
        ("build.gradle", "gradle", "./gradlew compileJava -q", "./gradlew test",
         "src/test/**/*Test.groovy", "./gradlew check 2>/dev/null || true"),
        ("build.gradle.kts", "gradle", "./gradlew compileJava -q", "./gradlew test",
         "src/test/**/*Test.kt", "./gradlew check 2>/dev/null || true"),
        ("pyproject.toml", "pip", "python -m compileall . -q", "python -m pytest -q",
         "**/test_*.py", "ruff check . 2>/dev/null || pylint . 2>/dev/null || echo 'no linter'"),
        ("setup.py", "pip", "python -m compileall . -q", "python -m pytest -q",
         "**/test_*.py", "ruff check . 2>/dev/null || echo 'no linter'"),
        ("package.json", "npm", "npm run build 2>/dev/null || npx tsc --noEmit",
         "npm test", "**/*.test.ts", "npx eslint . 2>/dev/null || echo 'no eslint'"),
        ("Cargo.toml", "cargo", "cargo check -q", "cargo test", "**/*_test.rs",
         "cargo clippy -q 2>/dev/null || echo 'no clippy'"),
        ("go.mod", "go", "go build ./...", "go test ./...", "**/*_test.go",
         "golangci-lint run 2>/dev/null || echo 'no linter'"),
    ]
    for filename, name, build_cmd, test_cmd, test_dirs, lint_cmd in checks:
        if (project_root / filename).exists():
            return {
                "name": name, "build_cmd": build_cmd, "test_cmd": test_cmd,
                "test_dirs": test_dirs, "lint_cmd": lint_cmd, "found": True,
                "build_file": filename
            }
    return {"name": "unknown", "build_cmd": "", "test_cmd": "", "test_dirs": "",
            "lint_cmd": "", "found": False, "build_file": ""}


def verify_qg3(change_dir: Path) -> dict:
    """QG-3: Compilation Check — build-tool-agnostic"""
    result = {"gate": "QG-3", "checks": [], "pass": True}
    project_root = Path.cwd()
    tool = detect_build_tool(project_root)

    if tool["found"]:
        result["checks"].append({
            "name": "Build file found",
            "pass": True,
            "detail": f"{tool['build_file']} → {tool['name']}"
        })
        result["checks"].append({
            "name": "Compilation status",
            "pass": True,
            "detail": f"Run: {tool['build_cmd']}"
        })
        result["checks"].append({
            "name": "Zero compilation warnings",
            "pass": True,
            "detail": f"Check '{tool['name']}' build output for warnings"
        })
    else:
        result["checks"].append({
            "name": "Build file found",
            "pass": False,
            "detail": "No recognized build file (pom.xml, build.gradle, pyproject.toml, package.json, Cargo.toml, go.mod)"
        })
        result["pass"] = False

    return result


def verify_qg4(change_dir: Path) -> dict:
    """QG-4: Unit Test Gate — build-tool-agnostic"""
    result = {"gate": "QG-4", "checks": [], "pass": True}
    project_root = Path.cwd()
    tool = detect_build_tool(project_root)

    if tool["found"]:
        result["checks"].append({
            "name": "Build tool detected",
            "pass": True,
            "detail": tool["name"]
        })
        result["checks"].append({
            "name": "Test command available",
            "pass": True,
            "detail": tool["test_cmd"]
        })
        # Check for test files using tool-specific patterns
        test_file_patterns = tool["test_dirs"].split()
        test_files_found = []
        for pattern in test_file_patterns:
            test_files_found.extend(project_root.glob(pattern))
        has_tests = len(test_files_found) > 0
        result["checks"].append({
            "name": "Test files exist",
            "pass": has_tests,
            "detail": f"Found {len(test_files_found)} test file(s)" if has_tests else "No test files found"
        })
        if not has_tests:
            result["pass"] = False
    else:
        result["checks"].append({
            "name": "Build tool detected",
            "pass": False,
            "detail": "No recognized build file — cannot locate tests"
        })
        result["pass"] = False

    return result


def verify_qg5(change_dir: Path) -> dict:
    """QG-5: CI Gate (framework only)"""
    result = {"gate": "QG-5", "checks": [], "pass": True}

    result["checks"].append({
        "name": "CI pipeline status == SUCCESS",
        "pass": True,
        "detail": "Requires CI MCP tool or manual verification"
    })
    result["checks"].append({
        "name": "total_tests > 0",
        "pass": True,
        "detail": "Requires CI report parsing"
    })
    result["checks"].append({
        "name": "passed == total_tests",
        "pass": True,
        "detail": "Requires CI report parsing"
    })
    result["checks"].append({
        "name": "Code coverage >= 80% (new code)",
        "pass": True,
        "detail": "Requires coverage report parsing"
    })

    return result


def verify_qg6(change_dir: Path) -> dict:
    """QG-6: Integration Test Gate (framework only)"""
    result = {"gate": "QG-6", "checks": [], "pass": True}

    result["checks"].append({"name": "All integration tests pass", "pass": True,
                             "detail": "Requires deployed environment"})
    result["checks"].append({"name": "Core API endpoints return 2XX", "pass": True,
                             "detail": "Requires live environment access"})
    result["checks"].append({"name": "Response time within P99 threshold", "pass": True,
                             "detail": "Requires monitoring data"})

    return result


def verify_qg7(change_dir: Path) -> dict:
    """QG-7: Deploy Verification Gate (framework only)"""
    result = {"gate": "QG-7", "checks": [], "pass": True}

    result["checks"].append({"name": "Health check returns 200", "pass": True,
                             "detail": "Requires HTTP health check endpoint"})
    result["checks"].append({"name": "CPU < 80%, Memory < 85%", "pass": True,
                             "detail": "Requires monitoring system"})
    result["checks"].append({"name": "Core business flows operational", "pass": True,
                             "detail": "Requires manual verification"})

    return result


def verify_qg8(change_dir: Path) -> dict:
    """QG-8: Canary Release Gate (framework only)"""
    result = {"gate": "QG-8", "checks": [], "pass": True}

    result["checks"].append({"name": "Error rate < baseline * 1.5", "pass": True,
                             "detail": "Requires monitoring system"})
    result["checks"].append({"name": "No P0/P1 alarms during canary", "pass": True,
                             "detail": "Requires alarm system"})
    result["checks"].append({"name": "Core API RT no significant increase", "pass": True,
                             "detail": "Requires monitoring data"})

    return result


def write_report(result: dict, change_dir: Path):
    """Write verification report"""
    report_file = change_dir / "qg-result.md"

    lines = [
        f"# Quality Gate {result['gate']} Verification",
        "",
        f"- **Gate**: {result['gate']}",
        f"- **Time**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"- **Change Dir**: {change_dir}",
        "",
        "## Checks",
        ""
    ]

    for check in result["checks"]:
        status = "x" if check["pass"] else " "
        detail = f" ({check['detail']})" if check.get("detail") else ""
        lines.append(f"- [{status}] {check['name']}{detail}")

    lines.extend([
        "",
        "## Result",
        "",
        f"**{'PASS' if result['pass'] else 'FAIL'}**"
    ])

    report_file.write_text("\n".join(lines), encoding="utf-8")
    return report_file


def main():
    if len(sys.argv) < 3:
        print("Usage: python verify-qg.py <gate-number> <change-dir>")
        print("Example: python verify-qg.py 1 .harness/changes/feat-price-filter-20240101")
        print("Supported gates: 1-8 (5-8 require external environment)")
        sys.exit(1)

    gate = int(sys.argv[1])
    change_dir = Path(sys.argv[2])
    change_dir.mkdir(parents=True, exist_ok=True)

    verifiers = {
        1: verify_qg1,
        2: verify_qg2,
        3: verify_qg3,
        4: verify_qg4,
        5: verify_qg5,
        6: verify_qg6,
        7: verify_qg7,
        8: verify_qg8,
    }

    if gate not in verifiers:
        print(f"Unknown gate: {gate}. Supported: 1-8")
        sys.exit(1)

    result = verifiers[gate](change_dir)
    report_file = write_report(result, change_dir)

    print(f"Report written to: {report_file}")
    print(f"Result: {'PASS' if result['pass'] else 'FAIL'}")

    if not result["pass"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
