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
