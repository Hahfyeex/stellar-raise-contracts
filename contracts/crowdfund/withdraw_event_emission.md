# Withdraw Event Emission

## Overview

This document describes the security-hardened event emission design for the
`withdraw()` function in the crowdfund contract. All event publishing is
centralised in `src/withdraw_event_emission.rs` to improve readability,
testability, and security.

## Problem

The original `withdraw()` implementation had two issues:

1. **Unbounded per-contributor events** — the NFT minting loop emitted one
   `nft_minted` event per contributor with no upper bound:

   ```rust
   for contributor in contributors.iter() {
       // ... mint NFT ...
       env.events().publish(("campaign", "nft_minted"), (contributor, token_id));
   }
   ```

   A campaign with thousands of contributors would emit thousands of events in
   a single transaction, causing unpredictable and potentially excessive gas
   consumption.

2. **Inline event publishing** — event payloads were scattered across `withdraw()`
   with no validation, making it easy to accidentally emit a zero-fee or
   zero-payout event that would mislead off-chain indexers.

## Solution

### Module: `withdraw_event_emission.rs`

Three validated helper functions replace all inline `env.events().publish()`
calls inside `withdraw()`:

| Function                  | Event topic 2      | Validation                        |
|---------------------------|--------------------|-----------------------------------|
| `emit_fee_transferred`    | `fee_transferred`  | Panics if `fee <= 0`              |
| `emit_nft_batch_minted`   | `nft_batch_minted` | Panics if `minted_count == 0`     |
| `emit_withdrawn`          | `withdrawn`        | Panics if `creator_payout <= 0`   |

### Constant: `MAX_NFT_MINT_BATCH`

```rust
pub const MAX_NFT_MINT_BATCH: u32 = 50;
```

Defined in `lib.rs`. Controls the maximum number of NFT mints per `withdraw()`
call. Changing this value requires a contract upgrade via `upgrade()` (admin-only).

### Changes to `withdraw()`

1. The NFT loop breaks after `MAX_NFT_MINT_BATCH` eligible contributors.
2. Per-contributor `nft_minted` events are replaced with a **single summary event**
   via `emit_nft_batch_minted`.
3. All three event calls now go through the validated helpers.

## Events Reference

| Topic 1    | Topic 2            | Data                   | Condition                                    |
|------------|--------------------|------------------------|----------------------------------------------|
| `campaign` | `fee_transferred`  | `(Address, i128)`      | Platform fee is configured and fee > 0       |
| `campaign` | `nft_batch_minted` | `u32`                  | NFT contract set and at least 1 mint done    |
| `campaign` | `withdrawn`        | `(Address, i128, u32)` | Always on successful withdraw                |

### `withdrawn` data tuple

```
(creator: Address, creator_payout: i128, nft_minted_count: u32)
```

> **Breaking change for indexers:** The old two-field tuple `(Address, i128)` is
> now a three-field tuple `(Address, i128, u32)`. Off-chain indexers must be
> updated to decode the new shape.

## Security Assumptions

- **Positive-only amounts** — `emit_fee_transferred` and `emit_withdrawn` assert
  that their amount arguments are strictly positive. A zero or negative value
  indicates a logic error upstream and will panic rather than silently emit a
  misleading event.
- **Non-empty batch** — `emit_nft_batch_minted` asserts `minted_count > 0`.
  The caller guards this with `if minted > 0` before calling the helper.
- **State-before-event ordering** — `emit_withdrawn` is called after
  `TotalRaised` is zeroed and `Status` is set to `Successful`. Off-chain
  indexers that read contract state on event receipt will observe the final
  consistent state.
- **Single emission** — `emit_withdrawn` is called exactly once per
  `withdraw()` invocation. The status guard (`panic!("campaign is not active")`)
  prevents a second call from reaching the emission point.
- **Cap does not skip contributors permanently** — `MAX_NFT_MINT_BATCH` limits
  a single `withdraw()` call only. If full batch minting is required, the NFT
  contract owner should implement a separate claim mechanism.

## Test Coverage

File: `contracts/crowdfund/src/withdraw_event_emission_test.rs`

### NFT minting cap

| Test | What it verifies |
|------|-----------------|
| `test_withdraw_mints_all_when_within_cap` | All contributors minted when count < cap |
| `test_withdraw_caps_minting_at_max_batch` | Only `MAX_NFT_MINT_BATCH` minted when count > cap |
| `test_withdraw_mints_exactly_at_cap_boundary` | Exact boundary: count == cap mints exactly cap |
| `test_withdraw_mints_single_contributor` | Single contributor minted correctly |

### `nft_batch_minted` event

| Test | What it verifies |
|------|-----------------|
| `test_withdraw_emits_single_batch_event` | Exactly one event emitted (not one per contributor) |
| `test_withdraw_no_batch_event_without_nft_contract` | No event when NFT contract not set |
| `test_withdraw_batch_event_data_equals_minted_count` | Event data equals actual mint count |
| `test_withdraw_batch_event_data_capped_at_max` | Event data equals cap when contributors > cap |

### `withdrawn` event

| Test | What it verifies |
|------|-----------------|
| `test_withdraw_emits_withdrawn_event_once` | Emitted exactly once |
| `test_withdraw_emits_withdrawn_event_without_nft` | Emitted even without NFT contract |
| `test_withdrawn_event_nft_count_zero_without_nft_contract` | nft_count field is 0 without NFT |
| `test_withdrawn_event_payout_equals_total_raised_no_fee` | Payout equals total raised when no fee |
| `test_withdrawn_event_payout_reflects_fee_deduction` | Payout is net of platform fee |

### `fee_transferred` event

| Test | What it verifies |
|------|-----------------|
| `test_withdraw_emits_fee_transferred_event` | Emitted when platform fee is configured |
| `test_withdraw_no_fee_event_without_platform_config` | Not emitted without platform config |

### Double-withdraw guard

| Test | What it verifies |
|------|-----------------|
| `test_double_withdraw_panics` | Second withdraw() panics with "campaign is not active" |

### Security unit tests (emit helpers)

| Test | What it verifies |
|------|-----------------|
| `test_emit_fee_transferred_panics_on_zero_fee` | Panics on fee == 0 |
| `test_emit_fee_transferred_panics_on_negative_fee` | Panics on fee < 0 |
| `test_emit_fee_transferred_succeeds_with_positive_fee` | Succeeds with fee > 0 |
| `test_emit_nft_batch_minted_panics_on_zero_count` | Panics on count == 0 |
| `test_emit_nft_batch_minted_succeeds_with_positive_count` | Succeeds with count > 0 |
| `test_emit_withdrawn_panics_on_zero_payout` | Panics on payout == 0 |
| `test_emit_withdrawn_panics_on_negative_payout` | Panics on payout < 0 |
| `test_emit_withdrawn_succeeds_with_valid_args` | Succeeds with valid args |
| `test_emit_withdrawn_allows_zero_nft_count` | nft_count of 0 is valid |
