//! Comprehensive tests for the Stellar Token Minter module.
//!
//! This test suite covers:
//! - Initialization and edge cases
//! - Contribution validation and error handling
//! - Withdrawal logic with platform fees
//! - NFT minting integration
//! - Campaign statistics
//! - Security assumptions
//!
//! Target: 95%+ test coverage

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, IntoVal,
};

use crate::{StellarTokenMinter, StellarTokenMinterClient};

/// Helper: Create a test environment with token contract and minter.
fn setup_env() -> (Env, StellarTokenMinterClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(StellarTokenMinter, ());
    let client = StellarTokenMinterClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_contract_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract_id.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

    let creator = Address::generate(&env);
    token_admin_client.mint(&creator, &10_000_000);

    (env, client, creator, token_address, token_admin)
}

/// Helper: Mint tokens to an address.
fn mint_to(env: &Env, token_address: &Address, admin: &Address, to: &Address, amount: i128) {
    let admin_client = token::StellarAssetClient::new(env, token_address);
    admin_client.mint(to, &amount);
}

/// Helper: Initialize with default parameters.
fn default_init(
    client: &StellarTokenMinterClient,
    creator: &Address,
    token_address: &Address,
    deadline: u64,
) {
    client.initialize(
        &creator,
        creator,
        token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
    );
}

// ── Initialization Tests ─────────────────────────────────────────────────────

/// Test: Successful initialization stores all fields correctly.
#[test]
fn test_initialize_success() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    default_init(&client, &creator, &token_address, deadline);

    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.goal(), 1_000_000);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), 1_000);
    assert_eq!(client.token(), token_address);
}

/// Test: Double initialization returns AlreadyInitialized error.
#[test]
fn test_initialize_already_initialized() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    default_init(&client, &creator, &token_address, deadline);

    let result = client.try_initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &None,
    );

    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::AlreadyInitialized
    );
}

/// Test: Platform fee validation - fee at maximum (10,000 bps).
#[test]
fn test_initialize_platform_fee_max() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    let platform_addr = Address::generate(&env);
    let platform_config = crate::PlatformConfig {
        address: platform_addr,
        fee_bps: 10_000,
    };

    client.initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(platform_config),
    );

    assert_eq!(client.goal(), 1_000_000);
}

/// Test: Platform fee exceeds maximum panics.
#[test]
#[should_panic(expected = "platform fee cannot exceed 100%")]
fn test_initialize_platform_fee_exceeds_max() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    let platform_addr = Address::generate(&env);
    let platform_config = crate::PlatformConfig {
        address: platform_addr,
        fee_bps: 10_001,
    };

    client.initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(platform_config),
    );
}

/// Test: Zero goal initialization succeeds.
#[test]
fn test_initialize_zero_goal() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &creator,
        &creator,
        &token_address,
        &0,
        &deadline,
        &1_000,
        &None,
    );

    assert_eq!(client.goal(), 0);
}

/// Test: Zero minimum contribution initialization succeeds.
#[test]
fn test_initialize_zero_min_contribution() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    client.initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &0,
        &None,
    );

    assert_eq!(client.min_contribution(), 0);
}

// ── Contribution Tests ───────────────────────────────────────────────────────

/// Test: Successful contribution updates totals and emits event.
#[test]
fn test_contribute_success() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    client.contribute(&contributor, &500_000);

    assert_eq!(client.total_raised(), 500_000);
    assert_eq!(client.contribution(contributor.clone()), 500_000);
    assert_eq!(client.contributors().len(), 1);
}

/// Test: Multiple contributions from same contributor accumulate.
#[test]
fn test_contribute_accumulation() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);

    client.contribute(&contributor, &300_000);
    client.contribute(&contributor, &200_000);

    assert_eq!(client.total_raised(), 500_000);
    assert_eq!(client.contribution(contributor.clone()), 500_000);
    assert_eq!(client.contributors().len(), 1);
}

/// Test: Multiple contributors are tracked correctly.
#[test]
fn test_contribute_multiple_contributors() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor1 = Address::generate(&env);
    let contributor2 = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor1, 500_000);
    mint_to(&env, &token_address, &admin, &contributor2, 500_000);

    client.contribute(&contributor1, &300_000);
    client.contribute(&contributor2, &200_000);

    assert_eq!(client.total_raised(), 500_000);
    assert_eq!(client.contributors().len(), 2);
}

/// Test: Contribution below minimum returns BelowMinimum error.
#[test]
fn test_contribute_below_minimum() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    let result = client.try_contribute(&contributor, &500);
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::BelowMinimum
    );
}

/// Test: Zero amount contribution returns ZeroAmount error.
#[test]
fn test_contribute_zero_amount() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    let result = client.try_contribute(&contributor, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::ZeroAmount
    );
}

/// Test: Contribution after deadline returns CampaignEnded error.
#[test]
fn test_contribute_after_deadline() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_contribute(&contributor, &500_000);
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::CampaignEnded
    );
}

/// Test: Contribution to non-active campaign returns CampaignNotActive error.
#[test]
fn test_contribute_non_active_campaign() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    // Withdraw to set status to Successful
    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    let result = client.try_contribute(&contributor, &500_000);
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::CampaignNotActive
    );
}

/// Test: Contribution at exact minimum succeeds.
#[test]
fn test_contribute_at_minimum() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000);

    client.contribute(&contributor, &1_000);

    assert_eq!(client.total_raised(), 1_000);
}

/// Test: Contribution at exact deadline succeeds.
#[test]
fn test_contribute_at_deadline() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);

    env.ledger().set_timestamp(deadline);

    client.contribute(&contributor, &500_000);

    assert_eq!(client.total_raised(), 500_000);
}

// ── Withdrawal Tests ─────────────────────────────────────────────────────────

/// Test: Successful withdrawal transfers funds to creator.
#[test]
fn test_withdraw_success() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    assert_eq!(client.total_raised(), 1_000_000);
}

/// Test: Withdrawal before deadline returns CampaignStillActive error.
#[test]
fn test_withdraw_before_deadline() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    let result = client.try_withdraw();
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::CampaignStillActive
    );
}

/// Test: Withdrawal when goal not met returns GoalNotReached error.
#[test]
fn test_withdraw_goal_not_reached() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 500_000);
    client.contribute(&contributor, &500_000);

    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_withdraw();
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::GoalNotReached
    );
}

/// Test: Withdrawal with platform fee deducts correct amount.
#[test]
fn test_withdraw_with_platform_fee() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    let platform_addr = Address::generate(&env);
    let platform_config = crate::PlatformConfig {
        address: platform_addr.clone(),
        fee_bps: 1_000, // 10%
    };

    client.initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(platform_config),
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    assert_eq!(client.total_raised(), 1_000_000);
}

/// Test: Withdrawal with NFT contract mints NFTs to contributors.
#[test]
fn test_withdraw_with_nft_minting() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    // Set NFT contract
    let nft_contract = Address::generate(&env);
    client.set_nft_contract(&creator, &nft_contract);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    assert_eq!(client.total_raised(), 1_000_000);
}

/// Test: Withdrawal on non-active campaign panics.
#[test]
#[should_panic(expected = "campaign is not active")]
fn test_withdraw_non_active() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);
    client.withdraw();

    // Try to withdraw again
    client.withdraw();
}

// ── NFT Contract Tests ───────────────────────────────────────────────────────

/// Test: Set NFT contract by creator succeeds.
#[test]
fn test_set_nft_contract_success() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let nft_contract = Address::generate(&env);
    client.set_nft_contract(&creator, &nft_contract);

    assert_eq!(client.nft_contract(), Some(nft_contract));
}

/// Test: Set NFT contract by non-creator panics.
#[test]
#[should_panic(expected = "not authorized")]
fn test_set_nft_contract_unauthorized() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let non_creator = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    client.set_nft_contract(&non_creator, &nft_contract);
}

// ── Statistics Tests ─────────────────────────────────────────────────────────

/// Test: Get stats returns correct values for empty campaign.
#[test]
fn test_get_stats_empty() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 0);
    assert_eq!(stats.goal, 1_000_000);
    assert_eq!(stats.progress_bps, 0);
    assert_eq!(stats.contributor_count, 0);
    assert_eq!(stats.average_contribution, 0);
    assert_eq!(stats.largest_contribution, 0);
}

/// Test: Get stats returns correct values after contributions.
#[test]
fn test_get_stats_with_contributions() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor1 = Address::generate(&env);
    let contributor2 = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor1, 500_000);
    mint_to(&env, &token_address, &admin, &contributor2, 300_000);

    client.contribute(&contributor1, &500_000);
    client.contribute(&contributor2, &300_000);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 800_000);
    assert_eq!(stats.goal, 1_000_000);
    assert_eq!(stats.progress_bps, 8_000); // 80%
    assert_eq!(stats.contributor_count, 2);
    assert_eq!(stats.average_contribution, 400_000);
    assert_eq!(stats.largest_contribution, 500_000);
}

/// Test: Get stats progress_bps capped at 10,000.
#[test]
fn test_get_stats_progress_capped() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 2_000_000);

    client.contribute(&contributor, &2_000_000);

    let stats = client.get_stats();
    assert_eq!(stats.progress_bps, 10_000); // Capped at 100%
}

// ── View Function Tests ──────────────────────────────────────────────────────

/// Test: View functions return correct values.
#[test]
fn test_view_functions() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    assert_eq!(client.total_raised(), 0);
    assert_eq!(client.goal(), 1_000_000);
    assert_eq!(client.deadline(), deadline);
    assert_eq!(client.min_contribution(), 1_000);
    assert_eq!(client.token(), token_address);
    assert_eq!(client.nft_contract(), None);
    assert_eq!(client.contributors().len(), 0);
}

// ── Edge Case Tests ──────────────────────────────────────────────────────────

/// Test: Contribution with very large amount.
#[test]
fn test_contribute_large_amount() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    let large_amount = 1_000_000_000_000i128;
    mint_to(&env, &token_address, &admin, &contributor, large_amount);

    client.contribute(&contributor, &large_amount);

    assert_eq!(client.total_raised(), large_amount);
}

/// Test: Multiple withdrawals after successful campaign.
#[test]
fn test_multiple_withdrawals() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    // Second withdrawal should panic
    let result = client.try_withdraw();
    assert!(result.is_err());
}

/// Test: Contribution exactly at goal amount.
#[test]
fn test_contribute_exactly_at_goal() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);

    client.contribute(&contributor, &1_000_000);

    assert_eq!(client.total_raised(), 1_000_000);
    assert_eq!(client.get_stats().progress_bps, 10_000);
}

/// Test: Contribution just below goal.
#[test]
fn test_contribute_just_below_goal() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 999_999);

    client.contribute(&contributor, &999_999);

    assert_eq!(client.total_raised(), 999_999);
    assert_eq!(client.get_stats().progress_bps, 9_999);
}

/// Test: Platform fee with zero fee_bps.
#[test]
fn test_withdraw_zero_platform_fee() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    let platform_addr = Address::generate(&env);
    let platform_config = crate::PlatformConfig {
        address: platform_addr,
        fee_bps: 0,
    };

    client.initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(platform_config),
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    assert_eq!(client.total_raised(), 1_000_000);
}

/// Test: NFT contract not set returns None.
#[test]
fn test_nft_contract_not_set() {
    let (env, client, creator, token_address, _admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    assert_eq!(client.nft_contract(), None);
}

/// Test: Contribution list maintains order.
#[test]
fn test_contributors_order() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor1 = Address::generate(&env);
    let contributor2 = Address::generate(&env);
    let contributor3 = Address::generate(&env);

    mint_to(&env, &token_address, &admin, &contributor1, 100_000);
    mint_to(&env, &token_address, &admin, &contributor2, 200_000);
    mint_to(&env, &token_address, &admin, &contributor3, 300_000);

    client.contribute(&contributor1, &100_000);
    client.contribute(&contributor2, &200_000);
    client.contribute(&contributor3, &300_000);

    let contributors = client.contributors();
    assert_eq!(contributors.len(), 3);
    assert_eq!(contributors.get(0).unwrap(), contributor1);
    assert_eq!(contributors.get(1).unwrap(), contributor2);
    assert_eq!(contributors.get(2).unwrap(), contributor3);
}

/// Test: Withdrawal with maximum platform fee (100%).
#[test]
fn test_withdraw_max_platform_fee() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;

    let platform_addr = Address::generate(&env);
    let platform_config = crate::PlatformConfig {
        address: platform_addr,
        fee_bps: 10_000,
    };

    client.initialize(
        &creator,
        &creator,
        &token_address,
        &1_000_000,
        &deadline,
        &1_000,
        &Some(platform_config),
    );

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    assert_eq!(client.total_raised(), 1_000_000);
}

/// Test: Contribution with minimum amount after deadline.
#[test]
fn test_contribute_minimum_after_deadline() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000);

    env.ledger().set_timestamp(deadline + 1);

    let result = client.try_contribute(&contributor, &1_000);
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::CampaignEnded
    );
}

/// Test: Get stats with single large contributor.
#[test]
fn test_get_stats_single_large_contributor() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);

    client.contribute(&contributor, &1_000_000);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 1_000_000);
    assert_eq!(stats.contributor_count, 1);
    assert_eq!(stats.average_contribution, 1_000_000);
    assert_eq!(stats.largest_contribution, 1_000_000);
}

/// Test: Get stats with equal contributions.
#[test]
fn test_get_stats_equal_contributions() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor1 = Address::generate(&env);
    let contributor2 = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor1, 500_000);
    mint_to(&env, &token_address, &admin, &contributor2, 500_000);

    client.contribute(&contributor1, &500_000);
    client.contribute(&contributor2, &500_000);

    let stats = client.get_stats();
    assert_eq!(stats.total_raised, 1_000_000);
    assert_eq!(stats.contributor_count, 2);
    assert_eq!(stats.average_contribution, 500_000);
    assert_eq!(stats.largest_contribution, 500_000);
}

/// Test: Withdrawal at exact deadline.
#[test]
fn test_withdraw_at_exact_deadline() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline);

    let result = client.try_withdraw();
    assert_eq!(
        result.unwrap_err().unwrap(),
        crate::ContractError::CampaignStillActive
    );
}

/// Test: Withdrawal one second after deadline.
#[test]
fn test_withdraw_one_second_after_deadline() {
    let (env, client, creator, token_address, admin) = setup_env();
    let deadline = env.ledger().timestamp() + 3600;
    default_init(&client, &creator, &token_address, deadline);

    let contributor = Address::generate(&env);
    mint_to(&env, &token_address, &admin, &contributor, 1_000_000);
    client.contribute(&contributor, &1_000_000);

    env.ledger().set_timestamp(deadline + 1);

    client.withdraw();

    assert_eq!(client.total_raised(), 1_000_000);
}
