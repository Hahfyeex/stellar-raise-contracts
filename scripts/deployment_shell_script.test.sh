#!/usr/bin/env bash
# @title   deployment_shell_script.test.sh
# @notice  Unit tests for deployment_shell_script.sh using a lightweight
#          bash test harness (no external dependencies required).
# @dev     Run: bash scripts/deployment_shell_script.test.sh
#          Exit 0 = all tests passed.

set -euo pipefail

SCRIPT="$(dirname "$0")/deployment_shell_script.sh"
PASS=0
FAIL=0

# ── Harness ──────────────────────────────────────────────────────────────────

assert_exit() {
  local desc="$1" expected="$2"; shift 2
  local actual=0
  "$@" &>/dev/null || actual=$?
  if [[ "$actual" -eq "$expected" ]]; then
    echo "  PASS  $desc"
    (( PASS++ )) || true
  else
    echo "  FAIL  $desc  (expected exit $expected, got $actual)"
    (( FAIL++ )) || true
  fi
}

assert_output_contains() {
  local desc="$1" pattern="$2"; shift 2
  local out
  out="$("$@" 2>&1)" || true
  if echo "$out" | grep -q "$pattern"; then
    echo "  PASS  $desc"
    (( PASS++ )) || true
  else
    echo "  FAIL  $desc  (pattern '$pattern' not found in output)"
    (( FAIL++ )) || true
  fi
}

# ── Source helpers only (skip main) ──────────────────────────────────────────
# We source the script with main() stubbed out so we can test individual functions.

# shellcheck source=/dev/null
SOURCING=1
eval "$(sed 's/^main "\$@"$/: # main stubbed/' "$SCRIPT")"

# ── Tests: require_tool ───────────────────────────────────────────────────────

echo ""
echo "=== require_tool ==="

assert_exit "passes for 'bash' (always present)" 0 \
  bash -c "$(declare -f require_tool die log); DEPLOY_LOG=/dev/null; require_tool bash"

assert_exit "exits 1 for missing tool" 1 \
  bash -c "$(declare -f require_tool die log); DEPLOY_LOG=/dev/null; require_tool __no_such_tool_xyz__"

# ── Tests: validate_args ──────────────────────────────────────────────────────

echo ""
echo "=== validate_args ==="

FUTURE=$(( $(date +%s) + 86400 ))

assert_exit "passes with valid args" 0 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR GTOKEN 1000 $FUTURE 10"

assert_exit "exits 2 when creator is empty" 2 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args '' GTOKEN 1000 $FUTURE 10"

assert_exit "exits 2 when token is empty" 2 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR '' 1000 $FUTURE 10"

assert_exit "exits 2 when goal is non-numeric" 2 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR GTOKEN abc $FUTURE 10"

assert_exit "exits 2 when goal is negative string" 2 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR GTOKEN -5 $FUTURE 10"

assert_exit "exits 2 when deadline is non-numeric" 2 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR GTOKEN 1000 'not-a-ts' 10"

assert_exit "exits 2 when deadline is in the past" 2 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR GTOKEN 1000 1 10"

assert_exit "exits 2 when min_contribution is non-numeric" 2 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR GTOKEN 1000 $FUTURE abc"

assert_exit "accepts min_contribution default of 1" 0 \
  bash -c "$(declare -f validate_args die log); DEPLOY_LOG=/dev/null
           validate_args GCREATOR GTOKEN 1000 $FUTURE 1"

# ── Tests: build_contract (cargo stubbed) ────────────────────────────────────

echo ""
echo "=== build_contract ==="

assert_exit "exits 3 when cargo build fails" 3 \
  bash -c "$(declare -f build_contract die log)
           DEPLOY_LOG=/dev/null
           WASM_PATH=/nonexistent.wasm
           cargo() { return 1; }
           build_contract"

assert_exit "exits 3 when WASM missing after successful build" 3 \
  bash -c "$(declare -f build_contract die log)
           DEPLOY_LOG=/dev/null
           WASM_PATH=/nonexistent.wasm
           cargo() { return 0; }
           build_contract"

assert_exit "passes when cargo succeeds and WASM exists" 0 \
  bash -c "$(declare -f build_contract die log)
           TMP=\$(mktemp); DEPLOY_LOG=/dev/null; WASM_PATH=\"\$TMP\"
           cargo() { return 0; }
           build_contract
           rm -f \"\$TMP\""

# ── Tests: deploy_contract (stellar stubbed) ─────────────────────────────────

echo ""
echo "=== deploy_contract ==="

assert_exit "exits 4 when stellar deploy fails" 4 \
  bash -c "$(declare -f deploy_contract die log)
           DEPLOY_LOG=/dev/null; WASM_PATH=/dev/null; NETWORK=testnet
           stellar() { return 1; }
           deploy_contract GCREATOR"

assert_exit "exits 4 when stellar returns empty contract ID" 4 \
  bash -c "$(declare -f deploy_contract die log)
           DEPLOY_LOG=/dev/null; WASM_PATH=/dev/null; NETWORK=testnet
           stellar() { echo ''; }
           deploy_contract GCREATOR"

assert_output_contains "returns contract ID on success" "CTEST123" \
  bash -c "$(declare -f deploy_contract die log)
           DEPLOY_LOG=/dev/null; WASM_PATH=/dev/null; NETWORK=testnet
           stellar() { echo 'CTEST123'; }
           deploy_contract GCREATOR"

# ── Tests: init_contract (stellar stubbed) ───────────────────────────────────

echo ""
echo "=== init_contract ==="

assert_exit "exits 5 when stellar invoke fails" 5 \
  bash -c "$(declare -f init_contract die log)
           DEPLOY_LOG=/dev/null; NETWORK=testnet
           stellar() { return 1; }
           init_contract CTEST GCREATOR GTOKEN 1000 $FUTURE 10"

assert_exit "passes when stellar invoke succeeds" 0 \
  bash -c "$(declare -f init_contract die log)
           DEPLOY_LOG=/dev/null; NETWORK=testnet
           stellar() { return 0; }
           init_contract CTEST GCREATOR GTOKEN 1000 $FUTURE 10"

# ── Tests: log output ────────────────────────────────────────────────────────

echo ""
echo "=== log / die ==="

assert_output_contains "log writes level tag" "\[INFO\]" \
  bash -c "$(declare -f log); DEPLOY_LOG=/dev/null; log INFO 'hello'"

assert_output_contains "log writes message" "hello world" \
  bash -c "$(declare -f log); DEPLOY_LOG=/dev/null; log INFO 'hello world'"

assert_exit "die exits with supplied code" 3 \
  bash -c "$(declare -f log die); DEPLOY_LOG=/dev/null; die 3 'boom'"

assert_output_contains "die logs ERROR level" "\[ERROR\]" \
  bash -c "$(declare -f log die); DEPLOY_LOG=/dev/null; die 3 'boom'" || true

# ── Tests: DEPLOY_LOG file capture ───────────────────────────────────────────

echo ""
echo "=== DEPLOY_LOG file capture ==="

assert_exit "log appends to DEPLOY_LOG file" 0 \
  bash -c "$(declare -f log)
           TMP=\$(mktemp); DEPLOY_LOG=\"\$TMP\"
           log INFO 'test entry'
           grep -q 'test entry' \"\$TMP\"
           rm -f \"\$TMP\""

_test_main_truncates_log() {
  local TMP_LOG TMP_SCRIPT FUTURE
  TMP_LOG=$(mktemp)
  TMP_SCRIPT=$(mktemp --suffix=.sh)
  FUTURE=$(( $(date +%s) + 86400 ))
  echo 'stale content' > "$TMP_LOG"

  # Build a self-contained script: real functions + stubbed externals + call main
  {
    # Stub cargo and stellar before sourcing so they override any PATH lookup
    echo 'cargo()   { return 0; }'
    echo 'stellar() { case "$2" in deploy) echo CXXX;; *) ;; esac; return 0; }'
    # Inline the deployment script with "main "$@"" replaced by a no-op
    sed 's/^main "\$@"$/: # stubbed/' "$SCRIPT"
    echo "main GCREATOR GTOKEN 1000 $FUTURE 1"
  } > "$TMP_SCRIPT"

  DEPLOY_LOG="$TMP_LOG" NETWORK=testnet bash "$TMP_SCRIPT" &>/dev/null
  local rc=$?
  rm -f "$TMP_SCRIPT"

  if [[ $rc -eq 0 ]] && ! grep -q 'stale content' "$TMP_LOG"; then
    rm -f "$TMP_LOG"
    return 0
  fi
  rm -f "$TMP_LOG"
  return 1
}
assert_exit "main truncates DEPLOY_LOG at start" 0 _test_main_truncates_log

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]] || exit 1
