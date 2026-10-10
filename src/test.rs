#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::Address as _,
    Address, Env, Vec,
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
        &150_0000000i128,
        &contributor,
        &2u32,
    );
    assert_eq!(milestone_id, 1);

    // 3. Contributor submits deliverable
    client.submit_milestone(&contributor, &bounty_id, &milestone_id, &symbol_short!("pr123"));

    // 4. Reviewer 1 approves -> status becomes UnderReview
    client.verify_milestone(&reviewer1, &bounty_id, &milestone_id, &VoteDecision::Approve);
    let m_state = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m_state.status, MilestoneStatus::UnderReview);
    assert_eq!(m_state.approvals, 1);

    // 5. Reviewer 2 approves -> threshold reached! Status becomes Approved
    client.verify_milestone(&reviewer2, &bounty_id, &milestone_id, &VoteDecision::Approve);
    let m_approved = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m_approved.status, MilestoneStatus::Approved);
    assert_eq!(m_approved.approvals, 2);

    // 6. Release conditional payment
    client.release_milestone_payment(&creator, &bounty_id, &milestone_id);

    // 7. Verify contributor received 150 XLM
    assert_eq!(token_client.balance(&contributor), 150_0000000i128);
    // Contract escrow remaining = 50 XLM
    assert_eq!(token_client.balance(&contract_id), 50_0000000i128);

    let m_paid = client.get_milestone(&bounty_id, &milestone_id);
    assert_eq!(m_paid.status, MilestoneStatus::Paid);
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
    let reviewer = Address::generate(&env);
    let contributor = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("VoteBty"), &100_0000000i128, &token_address);
    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("Mile1"), &50_0000000i128, &contributor, &2u32);

    client.submit_milestone(&contributor, &b_id, &m_id, &symbol_short!("pr1"));

    // First vote succeeds
    client.verify_milestone(&reviewer, &b_id, &m_id, &VoteDecision::Approve);

    // Second vote by same reviewer MUST fail!
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

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let contributor = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("Premature"), &100_0000000i128, &token_address);
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

// -------------------------------------------------------------
// PROGRAMMABLE SETTLEMENT ROUTER & ESCROW ADVANCED TESTS
// -------------------------------------------------------------

#[test]
fn test_multi_recipient_fixed_settlement() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, token_client) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let dev = Address::generate(&env);
    let designer = Address::generate(&env);
    let reviewer = Address::generate(&env);

    asset_client.mint(&funder, &1000_0000000i128);

    // 1. Create and fund 1,000 XLM bounty
    let b_id = client.create_bounty(&creator, &symbol_short!("Treasury"), &1000_0000000i128, &token_address);
    client.fund_bounty(&funder, &b_id, &1000_0000000i128);

    // 2. Milestone for 1,000 XLM with threshold = 1
    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("V3Feature"), &1000_0000000i128, &dev, &1u32);

    // 3. Configure Multi-Recipient Fixed Settlement:
    // Dev: 700 XLM, Designer: 200 XLM, Reviewer: 100 XLM (Total = 1,000 XLM)
    let mut shares = Vec::new(&env);
    shares.push_back(RecipientShare {
        recipient: dev.clone(),
        amount: 700_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("dev"),
    });
    shares.push_back(RecipientShare {
        recipient: designer.clone(),
        amount: 200_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("design"),
    });
    shares.push_back(RecipientShare {
        recipient: reviewer.clone(),
        amount: 100_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("review"),
    });

    client.configure_settlement(&creator, &b_id, &m_id, &AllocationType::Fixed, &shares);

    assert!(client.has_settlement(&b_id, &m_id));
    let settlement_config = client.get_settlement(&b_id, &m_id);
    assert_eq!(settlement_config.total_amount, 1000_0000000i128);
    assert_eq!(settlement_config.recipients.len(), 3);

    // 4. Submit and verify milestone
    client.submit_milestone(&dev, &b_id, &m_id, &symbol_short!("pr99"));
    let reviewer_voter = Address::generate(&env);
    client.verify_milestone(&reviewer_voter, &b_id, &m_id, &VoteDecision::Approve);

    let m_status = client.get_milestone(&b_id, &m_id);
    assert_eq!(m_status.status, MilestoneStatus::Approved);

    // 5. Execute settlement atomically
    client.execute_settlement(&creator, &b_id, &m_id);

    // 6. Verify exact disbursement to all 3 recipients
    assert_eq!(token_client.balance(&dev), 700_0000000i128);
    assert_eq!(token_client.balance(&designer), 200_0000000i128);
    assert_eq!(token_client.balance(&reviewer), 100_0000000i128);

    // Treasury balance reduced to 0
    assert_eq!(token_client.balance(&contract_id), 0);

    let final_settlement = client.get_settlement(&b_id, &m_id);
    assert_eq!(final_settlement.status, SettlementStatus::Settled);
    assert!(final_settlement.is_immutable);
}

#[test]
fn test_multi_recipient_percentage_settlement() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, token_client) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let dev = Address::generate(&env);
    let designer = Address::generate(&env);
    let reviewer = Address::generate(&env);

    asset_client.mint(&funder, &500_0000000i128);

    let b_id = client.create_bounty(&creator, &symbol_short!("SplitBty"), &500_0000000i128, &token_address);
    client.fund_bounty(&funder, &b_id, &500_0000000i128);

    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("MilePct"), &500_0000000i128, &dev, &1u32);

    // Configure 60% (6000 bps) / 25% (2500 bps) / 15% (1500 bps) = 10000 bps
    let mut shares = Vec::new(&env);
    shares.push_back(RecipientShare {
        recipient: dev.clone(),
        amount: 0,
        percentage_bps: 6000,
        label: symbol_short!("dev"),
    });
    shares.push_back(RecipientShare {
        recipient: designer.clone(),
        amount: 0,
        percentage_bps: 2500,
        label: symbol_short!("design"),
    });
    shares.push_back(RecipientShare {
        recipient: reviewer.clone(),
        amount: 0,
        percentage_bps: 1500,
        label: symbol_short!("qa"),
    });

    client.configure_settlement(&creator, &b_id, &m_id, &AllocationType::Percentage, &shares);

    // 500 XLM * 60% = 300 XLM; 25% = 125 XLM; 15% = 75 XLM
    let config = client.get_settlement(&b_id, &m_id);
    assert_eq!(config.recipients.get(0).unwrap().amount, 300_0000000i128);
    assert_eq!(config.recipients.get(1).unwrap().amount, 125_0000000i128);
    assert_eq!(config.recipients.get(2).unwrap().amount, 75_0000000i128);

    client.submit_milestone(&dev, &b_id, &m_id, &symbol_short!("pr77"));
    let rev_voter = Address::generate(&env);
    client.verify_milestone(&rev_voter, &b_id, &m_id, &VoteDecision::Approve);

    client.execute_settlement(&creator, &b_id, &m_id);

    assert_eq!(token_client.balance(&dev), 300_0000000i128);
    assert_eq!(token_client.balance(&designer), 125_0000000i128);
    assert_eq!(token_client.balance(&reviewer), 75_0000000i128);
}

#[test]
#[should_panic(expected = "Total fixed allocation does not match milestone reward amount")]
fn test_invalid_fixed_allocation_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let dev = Address::generate(&env);
    let designer = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("BadSplit"), &100_0000000i128, &token_address);
    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("M1"), &100_0000000i128, &dev, &1u32);

    let mut shares = Vec::new(&env);
    shares.push_back(RecipientShare {
        recipient: dev.clone(),
        amount: 60_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("dev"),
    });
    // 60 + 50 = 110 XLM != 100 XLM reward amount -> MUST PANIC!
    shares.push_back(RecipientShare {
        recipient: designer.clone(),
        amount: 50_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("des"),
    });

    client.configure_settlement(&creator, &b_id, &m_id, &AllocationType::Fixed, &shares);
}

#[test]
#[should_panic(expected = "Total percentage basis points must sum exactly to 10000 (100%)")]
fn test_invalid_percentage_basis_points_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let dev = Address::generate(&env);
    let designer = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("BadPct"), &100_0000000i128, &token_address);
    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("M1"), &100_0000000i128, &dev, &1u32);

    let mut shares = Vec::new(&env);
    shares.push_back(RecipientShare {
        recipient: dev.clone(),
        amount: 0,
        percentage_bps: 5000,
        label: symbol_short!("dev"),
    });
    // 5000 + 4000 = 9000 != 10000 -> MUST PANIC!
    shares.push_back(RecipientShare {
        recipient: designer.clone(),
        amount: 0,
        percentage_bps: 4000,
        label: symbol_short!("des"),
    });

    client.configure_settlement(&creator, &b_id, &m_id, &AllocationType::Percentage, &shares);
}

#[test]
#[should_panic(expected = "Duplicate recipient address not allowed")]
fn test_duplicate_recipient_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let dev = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("DupRecip"), &100_0000000i128, &token_address);
    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("M1"), &100_0000000i128, &dev, &1u32);

    let mut shares = Vec::new(&env);
    shares.push_back(RecipientShare {
        recipient: dev.clone(),
        amount: 50_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("dev"),
    });
    shares.push_back(RecipientShare {
        recipient: dev.clone(),
        amount: 50_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("dev2"),
    });

    client.configure_settlement(&creator, &b_id, &m_id, &AllocationType::Fixed, &shares);
}

#[test]
#[should_panic(expected = "Settlement has already been executed")]
fn test_double_settlement_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let dev = Address::generate(&env);
    let rev = Address::generate(&env);

    asset_client.mint(&funder, &100_0000000i128);

    let b_id = client.create_bounty(&creator, &symbol_short!("DoubleSt"), &100_0000000i128, &token_address);
    client.fund_bounty(&funder, &b_id, &100_0000000i128);

    let m_id = client.create_milestone(&creator, &b_id, &symbol_short!("M1"), &100_0000000i128, &dev, &1u32);

    let mut shares = Vec::new(&env);
    shares.push_back(RecipientShare {
        recipient: dev.clone(),
        amount: 100_0000000i128,
        percentage_bps: 0,
        label: symbol_short!("dev"),
    });
    client.configure_settlement(&creator, &b_id, &m_id, &AllocationType::Fixed, &shares);

    client.submit_milestone(&dev, &b_id, &m_id, &symbol_short!("pr1"));
    client.verify_milestone(&rev, &b_id, &m_id, &VoteDecision::Approve);

    // First execution succeeds
    client.execute_settlement(&creator, &b_id, &m_id);

    // Second execution MUST FAIL!
    client.execute_settlement(&creator, &b_id, &m_id);
}

#[test]
fn test_bounty_refund_recovery() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, token_client) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);

    asset_client.mint(&funder, &300_0000000i128);

    let b_id = client.create_bounty(&creator, &symbol_short!("RefundBty"), &300_0000000i128, &token_address);
    client.fund_bounty(&funder, &b_id, &300_0000000i128);

    assert_eq!(token_client.balance(&contract_id), 300_0000000i128);

    // Creator recovers/refunds eligible bounty
    client.refund_bounty(&creator, &b_id);

    // Contract transfers 300 XLM back to creator
    assert_eq!(token_client.balance(&creator), 300_0000000i128);
    assert_eq!(token_client.balance(&contract_id), 0);

    let bounty = client.get_bounty(&b_id);
    assert_eq!(bounty.status, BountyStatus::Refunded);
    assert_eq!(bounty.funded_amount, 0);
    assert!(client.is_refunded(&b_id));
}

#[test]
fn test_bounty_completion_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let dev = Address::generate(&env);
    let rev = Address::generate(&env);

    asset_client.mint(&funder, &200_0000000i128);

    let b_id = client.create_bounty(&creator, &symbol_short!("CompleteB"), &200_0000000i128, &token_address);
    client.fund_bounty(&funder, &b_id, &200_0000000i128);

    let m1 = client.create_milestone(&creator, &b_id, &symbol_short!("M1"), &100_0000000i128, &dev, &1u32);
    let m2 = client.create_milestone(&creator, &b_id, &symbol_short!("M2"), &100_0000000i128, &dev, &1u32);

    // Settle M1
    client.submit_milestone(&dev, &b_id, &m1, &symbol_short!("p1"));
    client.verify_milestone(&rev, &b_id, &m1, &VoteDecision::Approve);
    client.execute_settlement(&creator, &b_id, &m1);

    // Settle M2
    client.submit_milestone(&dev, &b_id, &m2, &symbol_short!("p2"));
    client.verify_milestone(&rev, &b_id, &m2, &VoteDecision::Approve);
    client.execute_settlement(&creator, &b_id, &m2);

    // Complete bounty
    client.complete_bounty(&creator, &b_id);

    let b_final = client.get_bounty(&b_id);
    assert_eq!(b_final.status, BountyStatus::Completed);
}

#[test]
#[should_panic(expected = "All milestones must be paid before marking bounty completed")]
fn test_bounty_completion_fails_if_milestone_unpaid() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, _, _) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let dev = Address::generate(&env);

    let b_id = client.create_bounty(&creator, &symbol_short!("NotDone"), &100_0000000i128, &token_address);
    client.create_milestone(&creator, &b_id, &symbol_short!("M1"), &100_0000000i128, &dev, &1u32);

    // Milestone 1 is still pending -> completing bounty MUST fail!
    client.complete_bounty(&creator, &b_id);
}

// -------------------------------------------------------------
// ISSUE #16 REGRESSION: duplicate deposit on an already-funded
// bounty is rejected with Error::BountyAlreadyFunded.
// -------------------------------------------------------------
#[test]
fn test_duplicate_deposit_rejected_with_bounty_already_funded() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BountyTreasuryContract, ());
    let client = BountyTreasuryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let (token_address, asset_client, token_client) = setup_test_token(&env, &admin);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);

    asset_client.mint(&funder, &200_0000000i128);

    let b_id = client.create_bounty(
        &creator,
        &symbol_short!("DupFund"),
        &100_0000000i128,
        &token_address,
    );

    // Full funding succeeds and flips the bounty to Funded.
    client.fund_bounty(&funder, &b_id, &100_0000000i128);
    let bounty = client.get_bounty(&b_id);
    assert_eq!(bounty.status, BountyStatus::Funded);

    // Duplicate deposit call MUST be rejected with the new error code.
    let res = client.try_fund_bounty(&funder, &b_id, &50_0000000i128);
    assert_eq!(res, Err(Ok(Error::BountyAlreadyFunded)));

    // Escrow and bounty state are untouched by the rejected call.
    assert_eq!(token_client.balance(&contract_id), 100_0000000i128);
    let bounty = client.get_bounty(&b_id);
    assert_eq!(bounty.funded_amount, 100_0000000i128);
    assert_eq!(bounty.status, BountyStatus::Funded);
}
