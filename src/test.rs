#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _},
    Address, Env,
};

fn setup_test_token<'a>(
    env: &'a Env,
    admin: &Address,
) -> (Address, token::StellarAssetClient<'a>, token::Client<'a>) {
    let token_address = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let asset_client = token::StellarAssetClient::new(env, &token_address);
    let token_client = token::Client::new(env, &token_address);
    (token_address, asset_client, token_client)
}

#[test]
fn test_initialize_and_create_bounty() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let title = symbol_short!("FixDocs");

    let bounty_id = client.create_bounty(&creator, &title, &100_0000000i128, &token_address);
    assert_eq!(bounty_id, 1);

    let bounty = client.get_bounty(&1);
    assert_eq!(bounty.id, 1);
    assert_eq!(bounty.target_amount, 100_0000000i128);
    assert_eq!(bounty.funded_amount, 0);
    assert_eq!(bounty.status, BountyStatus::Open);
}

#[test]
fn test_fund_bounty_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, token_client) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);

    asset_client.mint(&funder, &1000_0000000i128);

    let bounty_id = client.create_bounty(&creator, &symbol_short!("Bounty1"), &100_0000000i128, &token_address);

    // Fund 100 XLM into escrow
    client.fund_bounty(&funder, &bounty_id, &100_0000000i128);

    // Verify contract escrow holds the funds
    assert_eq!(token_client.balance(&contract_id), 100_0000000i128);

    let bounty = client.get_bounty(&bounty_id);
    assert_eq!(bounty.funded_amount, 100_0000000i128);
    assert_eq!(bounty.status, BountyStatus::Funded);
}

#[test]
fn test_end_to_end_milestone_verification_and_settlement() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, token_client) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let contributor = Address::generate(&env);
    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);

    asset_client.mint(&funder, &500_0000000i128);

    // 1. Create and fund bounty
    let bounty_id = client.create_bounty(&creator, &symbol_short!("CoreDev"), &200_0000000i128, &token_address);
    client.fund_bounty(&funder, &bounty_id, &200_0000000i128);

    // 2. Creator creates milestone with threshold = 2 approvals
    let milestone_id = client.create_milestone(
        &creator,
        &bounty_id,
        &symbol_short!("Mile1"),
        &75_0000000i128,
        &contributor,
        &2u32,
    );
    assert_eq!(milestone_id, 1);

    let m1 = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m1.status, MilestoneStatus::Pending);

    // 3. Contributor submits milestone with PR reference
    client.submit_milestone(&contributor, &bounty_id, &milestone_id, &symbol_short!("pr101"));

    let m1_submitted = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m1_submitted.status, MilestoneStatus::Submitted);
    assert_eq!(m1_submitted.submission_ref, symbol_short!("pr101"));

    // 4. First reviewer approves (1 / 2) -> status becomes UnderReview
    client.verify_milestone(&reviewer1, &bounty_id, &milestone_id, &VoteDecision::Approve);
    let m1_reviewed = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m1_reviewed.status, MilestoneStatus::UnderReview);
    assert_eq!(m1_reviewed.approvals, 1);

    // 5. Second reviewer approves (2 / 2) -> threshold reached, status becomes Approved!
    client.verify_milestone(&reviewer2, &bounty_id, &milestone_id, &VoteDecision::Approve);
    let m1_approved = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m1_approved.status, MilestoneStatus::Approved);
    assert_eq!(m1_approved.approvals, 2);

    // 6. Release payment: Authoritatively disburses 75 XLM to contributor
    let contributor_initial_balance = token_client.balance(&contributor);
    client.release_milestone_payment(&creator, &bounty_id, &milestone_id);

    // Contributor received 75 XLM
    assert_eq!(token_client.balance(&contributor), contributor_initial_balance + 75_0000000i128);
    // Contract escrow remaining = 125 XLM
    assert_eq!(token_client.balance(&contract_id), 125_0000000i128);

    let m1_paid = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m1_paid.status, MilestoneStatus::Paid);
}

#[test]
#[should_panic(expected = "Duplicate vote: reviewer has already voted on this milestone")]
fn test_duplicate_reviewer_vote_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let contributor = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("DupVote"), &100_0000000i128, &token_address);
    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("Mile1"), &50_0000000i128, &contributor, &2u32);
    client.submit_milestone(&contributor, &b_id, &m_id, &symbol_short!("pr1"));

    // First vote succeeds
    client.verify_milestone(&reviewer, &b_id, &m_id, &VoteDecision::Approve);
    // Duplicate vote from same reviewer MUST panic!
    client.verify_milestone(&reviewer, &b_id, &m_id, &VoteDecision::Approve);
}

#[test]
#[should_panic(expected = "Payment condition not satisfied: milestone has not reached approval threshold")]
fn test_payment_release_fails_if_not_approved() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let contributor = Address::generate(&env);

    asset_client.mint(&funder, &100_0000000i128);

    let b_id = client.create_bounty(&creator, &symbol_short!("LockTest"), &100_0000000i128, &token_address);
    client.fund_bounty(&funder, &b_id, &100_0000000i128);

    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("Mile1"), &50_0000000i128, &contributor, &3u32);
    client.submit_milestone(&contributor, &b_id, &m_id, &symbol_short!("pr1"));

    // Attempting to release before threshold (3 approvals) is reached MUST fail!
    client.release_milestone_payment(&creator, &b_id, &m_id);
}

#[test]
#[should_panic(expected = "Only designated recipient can submit milestone")]
fn test_unauthorized_contributor_submission_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let real_recipient = Address::generate(&env);
    let impostor = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("AuthTest"), &100_0000000i128, &token_address);
    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("Mile1"), &50_0000000i128, &real_recipient, &2u32);

    // Impostor trying to submit should panic!
    client.submit_milestone(&impostor, &b_id, &m_id, &symbol_short!("fake"));
}

#[test]
#[should_panic(expected = "Only bounty creator can add milestones")]
fn test_non_creator_cannot_add_milestones() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let stranger = Address::generate(&env);
    let contributor = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("OwnerTest"), &100_0000000i128, &token_address);

    client.create_milestone(&stranger, &b_id, &symbol_short!("BadMil"), &10_0000000i128, &contributor, &1u32);
}
