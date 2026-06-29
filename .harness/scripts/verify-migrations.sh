#!/usr/bin/env bash
# verify-migrations.sh
#
# Static checks for the SQL migration directory:
#   1. Every file matches the canonical naming convention.
#   2. Numbers form a contiguous range starting at 000001.
#   3. The reserved slot 012 is documented and never silently re-used.
#
# Usage:
#   bash .harness/scripts/verify-migrations.sh
#
# Exit codes:
#   0  — all checks passed
#   1  — at least one check failed (printed to stderr)
#
# This script is read-only. It does not modify files.

set -uo pipefail

# ─── Resolve project root (directory containing .harness/) ──────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MIG_DIR="$PROJECT_ROOT/crates/data-layer/migrations"

if [[ ! -d "$MIG_DIR" ]]; then
  echo "MIGRATIONS DIRECTORY NOT FOUND: $MIG_DIR" >&2
  exit 1
fi

# ─── Configuration ───────────────────────────────────────────────────────
# Canonical filename: YYYYMMDD (20240101, project bootstrap) + 6-digit
# sequential number (000001..) + snake_case subject + .sql.
PATTERN='^2024010100[0-9]{4}_[a-z][a-z0-9_]*\.sql$'
RESERVED_SLOTS=(12)  # 6-digit form: 000012 — see migration-naming.md

FAIL=0

# ─── 1. Filename format ──────────────────────────────────────────────────
shopt -s nullglob
files=( "$MIG_DIR"/*.sql )
shopt -u nullglob

if [[ ${#files[@]} -eq 0 ]]; then
  echo "NO MIGRATION FILES FOUND in $MIG_DIR" >&2
  exit 1
fi

bad=()
for f in "${files[@]}"; do
  name="$(basename "$f")"
  if [[ ! "$name" =~ $PATTERN ]]; then
    bad+=( "$name" )
  fi
done

if [[ ${#bad[@]} -gt 0 ]]; then
  echo "FAIL: migrations with non-canonical filenames:" >&2
  for n in "${bad[@]}"; do
    echo "  - $n" >&2
  done
  echo "  expected pattern: $PATTERN" >&2
  FAIL=1
fi

# ─── 2. Reserved slot check ──────────────────────────────────────────────
for slot in "${RESERVED_SLOTS[@]}"; do
  reserved_name="${MIG_DIR}/2024010100$(printf '%04d' "$slot")_"*.sql
  if compgen -G "$reserved_name" > /dev/null; then
    echo "FAIL: reserved slot 0$slot is taken by:" >&2
    ls $reserved_name >&2
    echo "  slot 0$slot is intentionally reserved (see .harness/wiki/migration-naming.md)" >&2
    FAIL=1
  fi
done

# ─── 3. Order / contiguity ───────────────────────────────────────────────
nums=()
for f in "${files[@]}"; do
  name="$(basename "$f")"
  if [[ "$name" =~ ^2024010100([0-9]{4})_[a-z][a-z0-9_]*\.sql$ ]]; then
    nums+=( "${BASH_REMATCH[1]}" )
  fi
done

# Sort numerically (10# forces base-10 interpretation, not octal)
sorted_nums=( $(printf '%s\n' "${nums[@]}" | sort -n) )

# Filter out reserved slots from the contiguity check
expected=1
for n in "${sorted_nums[@]}"; do
  n_dec=$((10#$n))
  # Advance `expected` past any reserved slot the loop is about to skip.
  for s in "${RESERVED_SLOTS[@]}"; do
    if [[ "$expected" == "$s" ]]; then
      (( expected++ ))
    fi
  done
  # Skip the current file if it IS a reserved slot.
  skip=0
  for s in "${RESERVED_SLOTS[@]}"; do
    if [[ "$n_dec" == "$s" ]]; then skip=1; break; fi
  done
  (( skip )) && continue
  if [[ "$n_dec" != "$expected" ]]; then
    printf 'FAIL: migration number gap — expected %06d, got %06d\n' "$expected" "$n_dec" >&2
    FAIL=1
    break
  fi
  (( expected++ ))
done

# ─── Result ─────────────────────────────────────────────────────────────
if [[ $FAIL -eq 0 ]]; then
  count=${#files[@]}
  echo "OK: $count migration file(s) passed all checks"
  exit 0
else
  echo "Migration verification FAILED. See messages above." >&2
  exit 1
fi
