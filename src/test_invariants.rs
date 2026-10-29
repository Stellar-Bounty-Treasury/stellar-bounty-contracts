#![cfg(test)]

use crate::{
    BountyTreasuryContract, BountyTreasuryContractClient,
    types::{AllocationType, DisputeResolution, RecipientShare, Role},
};
use soroban_sdk::{
    symbol_short,
    testutils::Address as _,
    token, Address, Env, Vec,
};

fn setup_test_env() -> (
    Env,
    BountyTreasuryContractClient<'static>,
    Address,
    Address,
    Address,
    token::StellarAssetClient<'static>,
    token::Client<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.initialize(&admin);

    let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(&env, &token_address);
    let token_client = token::Client::new(&env, &token_address);

    (env, client, admin, treasury, token_address, asset_client, token_client)
}

#[test]
fn test_invariant_reentrancy_lock() {
    let (_env, client, _admin, _treasury, _token, _asset, _tok_cli) = setup_test_env();
    let (major, minor, patch) = client.get_protocol_version();
    assert_eq!(major, 1);
    assert_eq!(minor, 2);
    assert_eq!(patch, 0);
}

#[test]
fn test_invariant_escrow_solvency() {
    let (env, client, _admin, _treasury, token_address, asset_client, _tok_cli) = setup_test_env();
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);

    asset_client.mint(&funder, &1000_0000000i128);

    let bounty_id = client.create_bounty(&creator, &symbol_short!("Bounty"), &1000_0000000i128, &token_address);
    client.fund_bounty(&funder, &bounty_id, &1000_0000000i128);

    let balance = client.get_bounty_escrow_balance(&bounty_id);
    assert!(balance >= 1000_0000000i128, "Solvency invariant violated: balance < funded");
}

#[test]
fn test_invariant_fee_cap_under_500_bps() {
    let (_env, client, admin, treasury, _token, _asset, _tok_cli) = setup_test_env();
    client.set_platform_fee(&admin, &500, &treasury);
    let (fee, rec) = client.get_platform_fee();
    assert_eq!(fee, 500);
    assert_eq!(rec, treasury);

    let res = client.try_set_platform_fee(&admin, &501, &treasury);
    assert!(res.is_err());
}

#[test]
fn test_invariant_settlement_sum_10000_bps() {
    let (env, client, _admin, _treasury, token_address, _asset, _tok_cli) = setup_test_env();
    let creator = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let bounty_id = client.create_bounty(&creator, &symbol_short!("Bounty"), &1000_0000000i128, &token_address);
    let m_id = client.create_milestone(&creator, &bounty_id, &symbol_short!("M1"), &1000_0000000i128, &recipient1, &100000);

    let mut recipients = Vec::new(&env);
    recipients.push_back(RecipientShare {
        recipient: recipient1,
        amount: 0,
        percentage_bps: 8000,
        label: symbol_short!("dev"),
    });
    recipients.push_back(RecipientShare {
        recipient: recipient2,
        amount: 0,
        percentage_bps: 1999, // sum is 9999, not 10000
        label: symbol_short!("qa"),
    });

    let res = client.try_configure_settlement(&creator, &bounty_id, &m_id, &AllocationType::Percentage, &recipients);
    assert!(res.is_err());
}

#[test]
fn test_invariant_storage_rent_ttl_extension() {
    let (_env, client, _admin, _treasury, _token, _asset, _tok_cli) = setup_test_env();
    client.extend_contract_ttl(&10000, &50000);
}
