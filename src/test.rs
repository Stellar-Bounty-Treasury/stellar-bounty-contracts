#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::Address as _,
    Address, Env,
};

#[test]
fn test_initialize_and_get_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_bounty_count(), 0);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialization_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    client.initialize(&admin1);
    client.initialize(&admin2);
}

#[test]
fn test_create_and_get_bounty() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let creator = Address::generate(&env);
    let title = symbol_short!("FixDocs");
    let target = 100_0000000i128; // 100 XLM in stroops

    let bounty_id = client.create_bounty(&creator, &title, &target);
    assert_eq!(bounty_id, 1);
    assert_eq!(client.get_bounty_count(), 1);

    let bounty = client.get_bounty(&1);
    assert_eq!(bounty.id, 1);
    assert_eq!(bounty.creator, creator);
    assert_eq!(bounty.title, title);
    assert_eq!(bounty.target_amount, target);
    assert_eq!(bounty.funded_amount, 0);
    assert_eq!(bounty.status, BountyStatus::Open);
}

#[test]
fn test_multiple_bounties_sequential_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);

    let id1 = client.create_bounty(&creator1, &symbol_short!("Bounty1"), &50_0000000i128);
    let id2 = client.create_bounty(&creator2, &symbol_short!("Bounty2"), &150_0000000i128);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(client.get_bounty_count(), 2);

    let b1 = client.get_bounty(&1);
    let b2 = client.get_bounty(&2);
    assert_eq!(b1.title, symbol_short!("Bounty1"));
    assert_eq!(b2.title, symbol_short!("Bounty2"));
}

#[test]
#[should_panic(expected = "Target amount must be positive")]
fn test_create_bounty_zero_target_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let creator = Address::generate(&env);
    let title = symbol_short!("ZeroBnty");
    client.create_bounty(&creator, &title, &0i128);
}

#[test]
#[should_panic(expected = "Target amount must be positive")]
fn test_create_bounty_negative_target_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let creator = Address::generate(&env);
    let title = symbol_short!("NegBnty");
    client.create_bounty(&creator, &title, &-500i128);
}

#[test]
#[should_panic(expected = "Bounty not found")]
fn test_get_nonexistent_bounty_fails() {
    let env = Env::default();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    client.get_bounty(&999);
}
