#!/usr/bin/env bash
# @title   deployment_shell_script.sh
# @notice  Builds, deploys, and initialises the Stellar Raise crowdfund contract
#          on a target network with structured error capturing and logging.
# @dev     Requires: stellar CLI (>=0.0.18), Rust + wasm32-unknown-unknown target.
#          All errors are captured to DEPLOY_LOG (default: deploy_errors.log).
#          Exit codes:
#            0  – success
#            1  – missing dependency
#            2  – invalid / missing argument
#            3  – build failure
#            4  – deploy failure
#            5  – initialise failure

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────

NETWORK="${NETWORK:-testnet}"
DEPLOY_LOG="${DEPLOY_LOG:-deploy_errors.log}"
WASM_PATH="target/wasm32-unknown-unknown/release/crowdfund.wasm"

# ── Helpers ──────────────────────────────────────────────────────────────────

# @notice Writes a timestamped message to stdout and the error log.
# @param  $1  severity  (INFO | WARN | ERROR)
# @param  $2  message
log() {
  local level="$1" msg="$2"
  local ts; ts="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "[$ts] [$level] $msg" | tee -a "$DEPLOY_LOG"
}

# @notice Logs an error and exits with the supplied code.
# @param  $1  exit_code
# @param  $2  message
die() {
  log "ERROR" "$2"
  exit "$1"
}

# @notice Verifies that a required CLI tool is present on PATH.
# @param  $1  tool name
require_tool() {
  command -v "$1" &>/dev/null || die 1 "Required tool not found: $1"
}

# ── Argument validation ───────────────────────────────────────────────────────

# @notice Validates all required positional arguments.
# @param  $1  creator   – Stellar address of the campaign creator
# @param  $2  token     – Stellar address of the token contract
# @param  $3  goal      – Funding goal (integer, stroops)
# @param  $4  deadline  – Unix timestamp for campaign end
# @param  $5  min_contribution – Minimum pledge amount (default: 1)
validate_args() {
  local creator="$1" token="$2" goal="$3" deadline="$4" min_contribution="$5"

  [[ -n "$creator" ]]          || die 2 "creator is required"
  [[ -n "$token" ]]            || die 2 "token is required"
  [[ "$goal" =~ ^[0-9]+$ ]]   || die 2 "goal must be a positive integer, got: '$goal'"
  [[ "$deadline" =~ ^[0-9]+$ ]] || die 2 "deadline must be a Unix timestamp, got: '$deadline'"
  [[ "$min_contribution" =~ ^[0-9]+$ ]] || die 2 "min_contribution must be a positive integer"

  local now; now="$(date +%s)"
  (( deadline > now )) || die 2 "deadline must be in the future (got $deadline, now $now)"
}

# ── Core steps ───────────────────────────────────────────────────────────────

# @notice Compiles the contract to WASM.
build_contract() {
  log "INFO" "Building WASM..."
  if ! cargo build --target wasm32-unknown-unknown --release 2>>"$DEPLOY_LOG"; then
    die 3 "cargo build failed – see $DEPLOY_LOG for details"
  fi
  [[ -f "$WASM_PATH" ]] || die 3 "WASM artifact not found at $WASM_PATH after build"
  log "INFO" "Build succeeded: $WASM_PATH"
}

# @notice Deploys the WASM to the network and returns the contract ID via stdout.
# @param  $1  source – signing identity / secret key
deploy_contract() {
  local source="$1"
  log "INFO" "Deploying to $NETWORK..."
  local contract_id
  if ! contract_id=$(stellar contract deploy \
      --wasm "$WASM_PATH" \
      --network "$NETWORK" \
      --source "$source" 2>>"$DEPLOY_LOG"); then
    die 4 "stellar contract deploy failed – see $DEPLOY_LOG for details"
  fi
  [[ -n "$contract_id" ]] || die 4 "Deploy returned an empty contract ID"
  log "INFO" "Contract deployed: $contract_id"
  echo "$contract_id"
}

# @notice Calls initialize on the deployed contract.
# @param  $1  contract_id
# @param  $2  creator
# @param  $3  token
# @param  $4  goal
# @param  $5  deadline
# @param  $6  min_contribution
init_contract() {
  local contract_id="$1" creator="$2" token="$3" goal="$4" deadline="$5" min_contribution="$6"
  log "INFO" "Initialising campaign on contract $contract_id..."
  if ! stellar contract invoke \
      --id "$contract_id" \
      --network "$NETWORK" \
      --source "$creator" \
      -- initialize \
      --creator "$creator" \
      --token "$token" \
      --goal "$goal" \
      --deadline "$deadline" \
      --min_contribution "$min_contribution" 2>>"$DEPLOY_LOG"; then
    die 5 "Contract initialisation failed – see $DEPLOY_LOG for details"
  fi
  log "INFO" "Campaign initialised successfully."
}

# ── Entry point ───────────────────────────────────────────────────────────────

main() {
  local creator="${1:-}"
  local token="${2:-}"
  local goal="${3:-}"
  local deadline="${4:-}"
  local min_contribution="${5:-1}"

  # Truncate log for this run
  : > "$DEPLOY_LOG"

  require_tool cargo
  require_tool stellar

  validate_args "$creator" "$token" "$goal" "$deadline" "$min_contribution"
  build_contract
  local contract_id
  contract_id="$(deploy_contract "$creator")"
  init_contract "$contract_id" "$creator" "$token" "$goal" "$deadline" "$min_contribution"

  echo ""
  echo "Contract ID: $contract_id"
  echo "Save this Contract ID for interacting with the campaign."
}

main "$@"
