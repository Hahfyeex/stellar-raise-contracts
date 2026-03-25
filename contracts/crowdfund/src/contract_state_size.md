# `contract_state_size` — Bounded Contract State for Reviewability and Reliability

## Overview

`contract_state_size` centralizes the limits for every crowdfund state field
whose size can grow from user input. The goal is to make worst-case storage
growth explicit, auditable, and enforceable in both local development and
CI/CD.

The module introduces pure validation helpers and wires them into the
contract's state-mutating entrypoints so oversize writes are rejected before
they are persisted.

## Why this matters

Without explicit bounds, a campaign can accumulate:

- Very large metadata strings
- Extremely long contributor or pledger indexes
- Unbounded roadmap entries
- Unbounded stretch-goal lists

That makes the contract harder to review, increases state- and payload-size
variance across environments, and weakens our confidence in worst-case
behavior during CI.

## Limits

| Constant                              | Value   | Purpose                                        |
|---------------------------------------|---------|------------------------------------------------|
| `MAX_CONTRIBUTORS`                    | `128`   | Max indexed contributor addresses              |
| `MAX_PLEDGERS`                        | `128`   | Max indexed pledger addresses                  |
| `MAX_ROADMAP_ITEMS`                   | `32`    | Max roadmap entries                            |
| `MAX_STRETCH_GOALS`                   | `32`    | Max stretch-goal milestones                    |
| `MAX_TITLE_LENGTH`                    | `128`   | Max campaign title size (bytes)                |
| `MAX_DESCRIPTION_LENGTH`              | `2048`  | Max campaign description size (bytes)          |
| `MAX_SOCIAL_LINKS_LENGTH`             | `512`   | Max social-links field size (bytes)            |
| `MAX_BONUS_GOAL_DESCRIPTION_LENGTH`   | `280`   | Max bonus-goal description size (bytes)        |
| `MAX_ROADMAP_DESCRIPTION_LENGTH`      | `280`   | Max roadmap-item description size (bytes)      |
| `MAX_METADATA_TOTAL_LENGTH`           | `2304`  | Combined title + description + socials budget  |

## Validation helpers

The module exposes small pure helpers so both contract code and tests can
reuse the same rules:

- `validate_title` - Validates title does not exceed MAX_TITLE_LENGTH
- `validate_description` - Validates description does not exceed MAX_DESCRIPTION_LENGTH
- `validate_social_links` - Validates social links do not exceed MAX_SOCIAL_LINKS_LENGTH
- `validate_bonus_goal_description` - Validates bonus goal description length
- `validate_roadmap_description` - Validates roadmap description length
- `validate_metadata_total_length` - Validates combined metadata budget (uses saturating arithmetic)
- `validate_contributor_capacity` - Validates contributor index has capacity
- `validate_pledger_capacity` - Validates pledger index has capacity
- `validate_roadmap_capacity` - Validates roadmap has capacity
- `validate_stretch_goal_capacity` - Validates stretch goals have capacity

Each helper returns `Result<(), &'static str>` and uses a stable error string
that makes failures easy to assert in tests and easy to spot in logs.

### Legacy compatibility functions

The module also provides legacy functions for backwards compatibility:

- `check_string_len` - Legacy string length checker (deprecated)
- `check_contributor_limit` - Legacy contributor limit checker (deprecated)
- `check_pledger_limit` - Legacy pledger limit checker (deprecated)
- `check_roadmap_limit` - Legacy roadmap limit checker (deprecated)
- `check_stretch_goal_limit` - Legacy stretch goal limit checker (deprecated)

These functions return `Result<(), StateSizeError>` and use the `StateSizeError` enum.

## Contract integration

The following entrypoints now enforce state-size limits:

### `initialize`

- Validates `bonus_goal_description` before storing it.

### `contribute`

- Rejects a contribution that would add a new address beyond
  `MAX_CONTRIBUTORS`.
- Existing contributors can still contribute even when the contributor index
  is already full.

### `pledge`

- Rejects a pledge that would add a new address beyond `MAX_PLEDGERS`.

### `update_metadata`

- Validates individual field lengths for `title`, `description`, and
  `socials`.
- Validates the combined metadata footprint using the existing stored values
  for fields that are not being updated in the current call.

### `add_roadmap_item`

- Rejects new entries once `MAX_ROADMAP_ITEMS` is reached.
- Rejects oversized roadmap descriptions.

### `add_stretch_goal`

- Rejects new milestones once `MAX_STRETCH_GOALS` is reached.

## Security assumptions

1. **DoS prevention**: Bounding state growth prevents attackers from flooding
   contributor/pledger lists until operations become too expensive.
2. **Gas safety**: Limiting indexed lists ensures `withdraw`, `refund`, and
   `collect_pledges` stay within Soroban resource limits.
3. **Storage integrity**: Individual string limits prevent oversized ledger
   entries that would cause host panics.
4. **Aggregate budget**: `MAX_METADATA_TOTAL_LENGTH` prevents campaigns from
   storing several individually-valid but collectively excessive fields.
5. **Existing participant access**: Contributor/pledger limits apply only to new
   index growth; existing participants are not locked out.

## NatSpec-style documentation

The Rust source includes NatSpec-style comments on:

- Every public constant (`@notice` tags)
- Every public validation helper (`@param`, `@return`, `@notice` tags)
- The module-level security assumptions and rationale
- Error types and their meanings

This keeps the rules close to the code and helps future reviews stay fast.

### Example NatSpec

```rust
/// Validates that a title does not exceed MAX_TITLE_LENGTH bytes.
///
/// @param title The title string to validate.
/// @return Ok(()) if the title is within limits, Err with descriptive message otherwise.
/// @notice Callers should treat errors as permanent rejections; the limit
///         will not change without a contract upgrade.
pub fn validate_title(title: &String) -> Result<(), &'static str> {
    // ...
}
```

## Test coverage

See [`contract_state_size.test.rs`](./contract_state_size.test.rs).

The dedicated suite covers:

### Pure helper tests
- Constant stability verification
- Exact-boundary acceptance for all string limits
- Rejection one byte/element over the limit for all fields
- Aggregate metadata budget acceptance and rejection
- Overflow-safe handling for aggregate-length calculations (saturating arithmetic)
- Collection-capacity acceptance and rejection for all list types

### Contract integration tests
- Contract-level rejection of oversize metadata
- Contract-level rejection when collections are full
- Contract-level acceptance at valid boundaries
- Existing contributor/pledger access when index is full

### Edge case coverage
- Zero-length inputs
- Maximum valid inputs
- Overflow attempts with saturating arithmetic
- Various combinations of metadata fields

## Review notes

This implementation is intentionally small:

- Limits live in one file
- Enforcement points are narrow and explicit
- Tests exercise both pure helpers and real contract calls
- NatSpec documentation enables automated tooling
- Saturating arithmetic prevents overflow attacks

That keeps the change efficient to review while still improving reliability
and reducing unbounded-state risk.

## Changelog

### Added in this release
- NatSpec-style documentation on all public APIs
- `MAX_PLEDGERS` limit constant
- `validate_pledger_capacity` helper function
- Additional boundary tests for acceptance cases
- Overflow-safe handling with saturating arithmetic
- Comprehensive documentation with security rationale
