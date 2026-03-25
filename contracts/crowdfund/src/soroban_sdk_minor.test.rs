//! Tests for `soroban_sdk_minor` frontend/scalability helpers.

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

use crate::soroban_sdk_minor::{
    assess_compatibility, clamp_page_size, emit_upgrade_audit_event,
    emit_upgrade_audit_event_with_note, pagination_window, validate_upgrade_note,
    validate_wasm_hash, CompatibilityStatus, FRONTEND_PAGE_SIZE_MAX, FRONTEND_PAGE_SIZE_MIN,
    SDK_VERSION_BASELINE, SDK_VERSION_TARGET, UPGRADE_NOTE_MAX_LEN,
};

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn make_string(env: &Env, len: u32, byte: u8) -> String {
    let bytes = [byte; 512];
    String::from_bytes(env, &bytes[..len as usize])
}

/// @notice Same-major bump stays compatible.
#[test]
fn compatibility_same_major_is_compatible() {
    let env = make_env();
    assert_eq!(
        assess_compatibility(&env, "22.0.0", "22.1.1"),
        CompatibilityStatus::Compatible
    );
}

/// @notice Cross-major bump requires migration.
#[test]
fn compatibility_cross_major_requires_migration() {
    let env = make_env();
    assert_eq!(
        assess_compatibility(&env, "22.1.0", "23.0.0"),
        CompatibilityStatus::RequiresMigration
    );
}

#[test]
fn version_constants_are_non_empty() {
    assert!(!SDK_VERSION_BASELINE.is_empty());
    assert!(!SDK_VERSION_TARGET.is_empty());
}

#[test]
fn validate_wasm_hash_rejects_zero() {
    let env = make_env();
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    assert!(!validate_wasm_hash(&hash));
}

#[test]
fn validate_wasm_hash_accepts_non_zero() {
    let env = make_env();
    let mut bytes = [0u8; 32];
    bytes[0] = 1;
    let hash = BytesN::from_array(&env, &bytes);
    assert!(validate_wasm_hash(&hash));
}

#[test]
fn clamp_page_size_enforces_bounds() {
    assert_eq!(clamp_page_size(0), FRONTEND_PAGE_SIZE_MIN);
    assert_eq!(clamp_page_size(1), 1);
    assert_eq!(clamp_page_size(50), 50);
    assert_eq!(clamp_page_size(FRONTEND_PAGE_SIZE_MAX + 1), FRONTEND_PAGE_SIZE_MAX);
}

#[test]
fn pagination_window_uses_clamped_limit() {
    let window = pagination_window(20, 1_000);
    assert_eq!(window.start, 20);
    assert_eq!(window.limit, FRONTEND_PAGE_SIZE_MAX);
}

#[test]
fn upgrade_note_validation_bounds() {
    let env = make_env();
    let ok = String::from_str(&env, "ok");
    assert!(validate_upgrade_note(&ok));

    let long = make_string(&env, UPGRADE_NOTE_MAX_LEN + 1, b'a');
    assert!(!validate_upgrade_note(&long));
}

#[test]
fn emit_audit_event_with_note_does_not_panic_for_valid_note() {
    let env = make_env();
    emit_upgrade_audit_event_with_note(
        &env,
        String::from_str(&env, "22.0.0"),
        String::from_str(&env, "22.0.1"),
        Address::generate(&env),
        String::from_str(&env, "frontend and indexer verified"),
    );
}

#[test]
#[should_panic(expected = "upgrade note exceeds UPGRADE_NOTE_MAX_LEN")]
fn emit_audit_event_with_note_panics_when_note_too_long() {
    let env = make_env();
    let long = make_string(&env, UPGRADE_NOTE_MAX_LEN + 1, b'x');
    emit_upgrade_audit_event_with_note(
        &env,
        String::from_str(&env, "22.0.0"),
        String::from_str(&env, "22.0.1"),
        Address::generate(&env),
        long,
    );
}

#[test]
fn emit_audit_event_without_note_does_not_panic() {
    let env = make_env();
    emit_upgrade_audit_event(
        &env,
        String::from_str(&env, "22.0.0"),
        String::from_str(&env, "22.0.1"),
        Address::generate(&env),
    );
}
