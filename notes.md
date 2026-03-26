# Notes

## How to Implement a New Edge Case for Soroban SDK Minor Version Bump — Gas Efficiency

This guide walks through adding a gas-efficiency edge case to the `soroban_sdk_minor` module
in `contracts/crowdfund/src/soroban_sdk_minor.rs`.

---

### 1. Understand the existing pattern

The module lives at:
- `contracts/crowdfund/src/soroban_sdk_minor.rs` — helpers
- `contracts/crowdfund/src/soroban_sdk_minor.test.rs` — tests

Existing helpers follow this shape:
- A pure function with clear inputs/outputs
- A `/// @notice` / `/// @dev` / `/// # Security` doc block
- A matching test in `soroban_sdk_minor.test.rs`

---

### 2. Add the gas-efficiency helper

Open `soroban_sdk_minor.rs` and add a constant + function. Example for a TTL-extension
gas guard (a common gas-efficiency edge case in minor bumps):

```rust
/// Maximum ledger TTL extension allowed per call to stay within gas budget.
pub const MAX_TTL_EXTENSION: u32 = 500_000;

/// @notice Clamp a requested TTL extension to the gas-safe maximum.
/// @dev    Soroban charges per-ledger for TTL extensions; unbounded values
///         can exhaust the gas budget silently. Clamp before calling
///         `env.storage().instance().extend_ttl(...)`.
/// @param  requested – The caller-supplied extension in ledgers.
/// @return The clamped, gas-safe extension value.
pub fn clamp_ttl_extension(requested: u32) -> u32 {
    requested.min(MAX_TTL_EXTENSION)
}
```

If the edge case involves the `Env`, follow the same signature style as
`assess_compatibility` — take `env: &Env` as the first argument.

---

### 3. Export it (if needed)

If `lib.rs` re-exports from this module, add your new symbol there. Check
`contracts/crowdfund/src/lib.rs` for any explicit `pub use` lines.

---

### 4. Write the test

In `soroban_sdk_minor.test.rs`, import your new symbol and add tests for:
- The happy path (value within bounds)
- The boundary value (exactly at the limit)
- The over-limit case (value exceeds the cap)

```rust
use crate::soroban_sdk_minor::{clamp_ttl_extension, MAX_TTL_EXTENSION};

#[test]
fn ttl_extension_clamps_to_max() {
    assert_eq!(clamp_ttl_extension(MAX_TTL_EXTENSION + 1), MAX_TTL_EXTENSION);
}

#[test]
fn ttl_extension_allows_values_within_limit() {
    assert_eq!(clamp_ttl_extension(1_000), 1_000);
}

#[test]
fn ttl_extension_boundary_is_inclusive() {
    assert_eq!(clamp_ttl_extension(MAX_TTL_EXTENSION), MAX_TTL_EXTENSION);
}
```

---

### 5. Run the tests

```bash
cargo test --package crowdfund soroban_sdk_minor -- --nocapture
```

All existing tests must still pass — the module has no breaking changes between
same-major SDK bumps (`assess_compatibility` returns `Compatible` for same-major).

---

### 6. Update the spec doc

Add a line to `contracts/crowdfund/soroban_sdk_minor.md` under **Implemented updates**:

```
- Added gas-efficiency TTL extension guard:
  - `MAX_TTL_EXTENSION`
  - `clamp_ttl_extension(...)`
```

---

### Key rules to follow

- Keep helpers pure where possible (no `env` mutation, no storage writes).
- Use `saturating_*` or `.min()` / `.clamp()` — never raw arithmetic that can overflow.
- Every public function needs a `@notice` doc comment.
- Same-major SDK bumps must not change storage keys or ABI — verify with `assess_compatibility`.
- Gas-sensitive paths (TTL, event payloads, page sizes) must always be bounded.

---

## How to Add Logging Bounds to Admin Upgrade Mechanism Validation — Security

This guide covers adding bounded, security-aware logging to the admin upgrade validation
path in `contracts/crowdfund/src/admin_upgrade_mechanism.rs`.

---

### 1. Understand the current validation flow

`validate_admin_upgrade` in `admin_upgrade_mechanism.rs` does two things:
1. Reads the admin from instance storage
2. Calls `admin.require_auth()`

`perform_upgrade` then calls `env.deployer().update_current_contract_wasm(...)`.

Neither function currently emits an event. Adding bounded logging here creates an
immutable on-chain audit trail for every upgrade attempt.

---

### 2. Why "bounded" matters for security

Soroban events have payload size limits. Unbounded string data in event topics or
values can:
- Exceed ledger limits and cause a runtime panic
- Bloat the event stream and degrade indexer/frontend performance
- Leak sensitive data if note fields are not validated before emission

Apply the same `UPGRADE_NOTE_MAX_LEN` guard already used in `soroban_sdk_minor.rs`.

---

### 3. Add the bounded audit event helper

In `admin_upgrade_mechanism.rs`, add an event emitter that validates payload size
before publishing:

```rust
use soroban_sdk::{Address, BytesN, Env, String, Symbol};

/// Maximum byte length for an upgrade audit note.
/// Mirrors `UPGRADE_NOTE_MAX_LEN` in soroban_sdk_minor to keep event
/// payloads consistent and indexer-friendly.
pub const UPGRADE_AUDIT_NOTE_MAX_LEN: u32 = 256;

/// @notice Emit a bounded upgrade-attempt audit event.
/// @dev    Validates note length before publishing to prevent oversized
///         event payloads that can panic or degrade indexer performance.
/// @param  env       – The Soroban environment.
/// @param  admin     – The address that attempted the upgrade.
/// @param  wasm_hash – The target WASM hash.
/// @param  note      – Optional audit note; must be <= UPGRADE_AUDIT_NOTE_MAX_LEN bytes.
///
/// # Security
/// - Panics on oversized note rather than silently truncating, so callers
///   cannot bypass the bound by passing garbage data.
/// - Does not log the old WASM hash to avoid leaking internal state.
pub fn emit_upgrade_attempt_event(
    env: &Env,
    admin: &Address,
    wasm_hash: &BytesN<32>,
    note: &String,
) {
    if note.len() > UPGRADE_AUDIT_NOTE_MAX_LEN {
        panic!("upgrade audit note exceeds UPGRADE_AUDIT_NOTE_MAX_LEN");
    }
    env.events().publish(
        (
            Symbol::new(env, "admin_upgrade"),
            Symbol::new(env, "attempt"),
        ),
        (admin.clone(), wasm_hash.clone(), note.clone()),
    );
}
```

---

### 4. Wire it into the upgrade path

Call the emitter inside `perform_upgrade` (or a new wrapper) after auth passes
but before the WASM swap, so a failed upgrade still leaves a trace:

```rust
pub fn perform_upgrade(env: &Env, admin: &Address, new_wasm_hash: BytesN<32>, note: String) {
    // Emit audit event first — if the WASM update panics, the event is
    // still recorded in the transaction's event stream.
    emit_upgrade_attempt_event(env, admin, &new_wasm_hash, &note);
    env.deployer().update_current_contract_wasm(new_wasm_hash);
}
```

---

### 5. Write the security-focused tests

In `admin_upgrade_mechanism.test.rs`, add tests that cover the logging bounds:

```rust
use soroban_sdk::{testutils::Events, Address, BytesN, Env, String};
use crate::admin_upgrade_mechanism::{
    emit_upgrade_attempt_event, UPGRADE_AUDIT_NOTE_MAX_LEN,
};

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Audit event is emitted for a valid upgrade attempt.
#[test]
fn upgrade_audit_event_emitted_on_valid_note() {
    let env = make_env();
    let admin = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[1u8; 32]);
    let note = String::from_str(&env, "routine patch");

    emit_upgrade_attempt_event(&env, &admin, &hash, &note);

    assert!(!env.events().all().is_empty());
}

/// Note at exactly the limit is accepted.
#[test]
fn upgrade_audit_note_at_boundary_is_accepted() {
    let env = make_env();
    let admin = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[2u8; 32]);
    // Build a string of exactly UPGRADE_AUDIT_NOTE_MAX_LEN bytes
    let bytes = vec![b'a'; UPGRADE_AUDIT_NOTE_MAX_LEN as usize];
    let note = String::from_bytes(&env, &bytes);

    emit_upgrade_attempt_event(&env, &admin, &hash, &note); // must not panic
}

/// Note one byte over the limit is rejected.
#[test]
#[should_panic(expected = "upgrade audit note exceeds UPGRADE_AUDIT_NOTE_MAX_LEN")]
fn upgrade_audit_note_over_limit_panics() {
    let env = make_env();
    let admin = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[3u8; 32]);
    let bytes = vec![b'x'; UPGRADE_AUDIT_NOTE_MAX_LEN as usize + 1];
    let note = String::from_bytes(&env, &bytes);

    emit_upgrade_attempt_event(&env, &admin, &hash, &note);
}

/// Zero-hash upgrade attempt is still logged before the hash guard fires.
#[test]
fn upgrade_audit_event_emitted_even_for_zero_hash() {
    let env = make_env();
    let admin = Address::generate(&env);
    let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
    let note = String::from_str(&env, "zero hash attempt");

    // Logging itself should not care about hash validity — that is the
    // caller's responsibility. The event provides the audit trail.
    emit_upgrade_attempt_event(&env, &admin, &zero_hash, &note);
    assert!(!env.events().all().is_empty());
}
```

---

### 6. Run the tests

```bash
cargo test --package crowdfund admin_upgrade -- --nocapture
```

---

### 7. Update the spec doc

Add to `contracts/crowdfund/admin_upgrade_mechanism.md` under **Security Features**:

```
- Bounded upgrade audit logging:
  - `UPGRADE_AUDIT_NOTE_MAX_LEN` — caps event payload size
  - `emit_upgrade_attempt_event(...)` — emits before WASM swap for full trace
```

---

### Key security rules to follow

- Emit the audit event before the WASM swap, not after — a failed swap still needs a trace.
- Never log the old WASM hash in the event payload; it leaks internal state.
- Panic on oversized notes — silent truncation hides bad input from auditors.
- Keep the note field optional at the call site; pass an empty `String` when no note is needed.
- The logging helper must be pure with respect to storage — no reads or writes, only `env.events().publish(...)`.
