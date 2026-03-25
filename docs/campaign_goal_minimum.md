# Campaign Goal Minimum Threshold Enforcement

## Overview

The `campaign_goal_minimum` module enforces a minimum financial goal for every
new crowdfunding campaign before it can be initialized on-chain.

### Why this enforcement is necessary for scalability

Without a minimum goal floor, the contract would accept campaigns with a goal
of zero or one token unit. This creates several problems at scale:

- **Ledger bloat** — Each campaign occupies at least one persistent ledger
  entry. Dust campaigns (goal ≈ 0) provide no economic value but consume the
  same storage as legitimate campaigns, increasing state size and validator
  overhead across the network.
- **Immediate drain exploit** — A zero-goal campaign is "successful" the
  moment any contribution arrives. The creator can call `withdraw()` instantly,
  turning the contract into a trivial donation drain with no accountability.
- **Spam / griefing** — Without a floor, an adversary can flood the factory
  with thousands of worthless campaigns at minimal cost, degrading indexer and
  frontend performance for all users.

Setting `MIN_GOAL_AMOUNT = 1` is deliberately permissive for test environments
while still closing the zero-goal attack surface. Governance can raise this
value (see [Configuration](#configuration) below).

---

## Configuration

`MIN_GOAL_AMOUNT` is defined as a compile-time constant in
`contracts/crowdfund/src/campaign_goal_minimum.rs`:

```rust
pub const MIN_GOAL_AMOUNT: i128 = 1;
```

### Updating the threshold

Because the constant is baked into the WASM binary, changing it requires a
contract upgrade:

1. Update `MIN_GOAL_AMOUNT` in `campaign_goal_minimum.rs`.
2. Build the new WASM:
   ```bash
   cargo build --release --target wasm32-unknown-unknown -p crowdfund
   ```
3. Upload and upgrade via the admin mechanism (see
   `contracts/crowdfund/admin_upgrade_mechanism.md`).

> **Governance note** — If the project adopts on-chain governance, the minimum
> threshold can be moved to contract storage (a `DataKey::MinGoalAmount` entry)
> and updated via a governance proposal without a full upgrade. The
> `validate_goal_amount` function signature already accepts `&Env` for exactly
> this future extension.

---

## Integration

### How `campaign_factory` (and `lib.rs`) should call this validation

Import the typed validator at the top of the calling module:

```rust
use crate::campaign_goal_minimum::validate_goal_amount;
```

Call it inside `initialize()` **before** any state is written, so a rejected
goal leaves no partial storage entries:

```rust
pub fn initialize(
    env: Env,
    goal: i128,
    // … other params
) -> Result<(), ContractError> {
    // Reject below-threshold goals atomically — no side-effects on failure.
    validate_goal_amount(&env, goal)?;

    // … rest of initialization
    Ok(())
}
```

The `?` operator propagates `ContractError::GoalTooLow` to the caller without
any additional boilerplate.

### Off-chain / SDK usage

The string-returning `validate_goal` helper is available for off-chain tooling
that does not want to depend on `ContractError`:

```rust
use crowdfund::campaign_goal_minimum::validate_goal;

validate_goal(proposed_goal).map_err(|e| anyhow::anyhow!(e))?;
```

---

## Security Assumptions

| Assumption | Detail |
|---|---|
| **Dust campaign prevention** | `MIN_GOAL_AMOUNT >= 1` ensures every campaign has a non-trivial economic commitment, preventing ledger-entry spam. |
| **No integer overflow** | The validation is a single signed comparison (`goal_amount < MIN_GOAL_AMOUNT`). No arithmetic is performed, so overflow is impossible regardless of the input value. |
| **Negative goal rejection** | `i128` can represent negative values. The `< MIN_GOAL_AMOUNT` check (where `MIN_GOAL_AMOUNT = 1`) rejects all negative and zero goals without a separate branch. |
| **Atomic rejection** | `validate_goal_amount` is called before any `env.storage()` writes in `initialize()`. A rejected goal produces no ledger mutations — the transaction reverts cleanly. |
| **Upgrade safety** | All contract storage and state persist across WASM upgrades. Raising `MIN_GOAL_AMOUNT` in a new binary does not affect already-initialized campaigns; it only applies to new `initialize()` calls. |

---

## API Reference

### `validate_goal_amount(env: &Env, goal_amount: i128) -> Result<(), ContractError>`

On-chain enforcement entry point. Returns `ContractError::GoalTooLow` when
`goal_amount < MIN_GOAL_AMOUNT`.

### `validate_goal(goal: i128) -> Result<(), &'static str>`

Off-chain / tooling helper. Returns a descriptive string error instead of
`ContractError` to avoid pulling in the full contract dependency.

### `MIN_GOAL_AMOUNT: i128`

Compile-time minimum campaign goal (currently `1`).

---

## Test Coverage

See [`contracts/crowdfund/src/campaign_goal_minimum_test.rs`](../contracts/crowdfund/src/campaign_goal_minimum_test.rs).

Key cases for `validate_goal_amount`:

| Test | Input | Expected |
|---|---|---|
| Exact threshold | `MIN_GOAL_AMOUNT` (1) | `Ok(())` |
| Well above threshold | `1_000_000_000` | `Ok(())` |
| One below threshold | `MIN_GOAL_AMOUNT - 1` (0) | `Err(GoalTooLow)` |
| Zero | `0` | `Err(GoalTooLow)` |
| Negative | `-1`, `i128::MIN` | `Err(GoalTooLow)` |
