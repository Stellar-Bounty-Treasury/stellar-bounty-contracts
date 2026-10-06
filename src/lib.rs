#![no_std]
use soroban_sdk::{
    contract, contractimpl, symbol_short, token, Address, Env, Symbol,
};

pub mod types;

#[cfg(test)]
mod test;

use types::{Bounty, BountyStatus, DataKey, Milestone, MilestoneStatus, VoteDecision};

#[contract]
pub struct BountyTreasuryContract;

#[contractimpl]
impl BountyTreasuryContract {
    /// Initialize the Bounty Treasury contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::BountyCounter, &0u64);
    }

    /// Retrieve the contract administrator address.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized")
    }

    /// Retrieve the total number of bounties created.
    pub fn get_bounty_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::BountyCounter)
            .unwrap_or(0u64)
    }

    /// 1. Create a new bounty with target amount and designated token (SAC / XLM).
    pub fn create_bounty(
        env: Env,
        creator: Address,
        title: Symbol,
        target_amount: i128,
        token: Address,
    ) -> u64 {
        creator.require_auth();

        if target_amount <= 0 {
            panic!("Target amount must be positive");
        }

        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::BountyCounter)
            .unwrap_or(0u64);

        counter += 1;

        let bounty = Bounty {
            id: counter,
            creator: creator.clone(),
            title,
            target_amount,
            funded_amount: 0,
            token,
            milestone_count: 0,
            status: BountyStatus::Open,
        };

        env.storage().persistent().set(&DataKey::Bounty(counter), &bounty);
        env.storage().instance().set(&DataKey::BountyCounter, &counter);

        // Emit structured event
        env.events().publish(
            (symbol_short!("bounty"), symbol_short!("created")),
            (counter, creator, target_amount),
        );

        counter
    }

    /// 2. Fund bounty escrow vault (transfers tokens from funder to contract).
    pub fn fund_bounty(env: Env, funder: Address, bounty_id: u64, amount: i128) {
        funder.require_auth();

        if amount <= 0 {
            panic!("Funding amount must be positive");
        }

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.status == BountyStatus::Cancelled || bounty.status == BountyStatus::Completed {
            panic!("Cannot fund cancelled or completed bounty");
        }

        // Authoritatively lock tokens in contract escrow
        let token_client = token::Client::new(&env, &bounty.token);
        token_client.transfer(&funder, &env.current_contract_address(), &amount);

        bounty.funded_amount += amount;

        if bounty.funded_amount >= bounty.target_amount {
            bounty.status = BountyStatus::Funded;
        }

        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);

        // Emit structured event
        env.events().publish(
            (symbol_short!("bounty"), symbol_short!("funded")),
            (bounty_id, funder, amount),
        );
    }

    /// 3. Create an on-chain milestone with allocation and verification threshold.
    pub fn create_milestone(
        env: Env,
        creator: Address,
        bounty_id: u64,
        description: Symbol,
        reward_amount: i128,
        recipient: Address,
        approval_threshold: u32,
    ) -> u32 {
        creator.require_auth();

        if reward_amount <= 0 {
            panic!("Milestone reward must be positive");
        }
        if approval_threshold == 0 {
            panic!("Approval threshold must be greater than zero");
        }

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != creator {
            panic!("Only bounty creator can add milestones");
        }

        bounty.milestone_count += 1;
        let milestone_id = bounty.milestone_count;

        let milestone = Milestone {
            id: milestone_id,
            bounty_id,
            description,
            reward_amount,
            recipient,
            status: MilestoneStatus::Pending,
            approval_threshold,
            approvals: 0,
            rejections: 0,
            submission_ref: symbol_short!("none"),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(bounty_id, milestone_id), &milestone);
        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);

        // Emit structured event
        env.events().publish(
            (symbol_short!("milestone"), symbol_short!("created")),
            (bounty_id, milestone_id, reward_amount),
        );

        milestone_id
    }

    /// 4. Submit milestone deliverable reference (GitHub PR / commit / CID).
    pub fn submit_milestone(
        env: Env,
        caller: Address,
        bounty_id: u64,
        milestone_id: u32,
        submission_ref: Symbol,
    ) {
        caller.require_auth();

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(bounty_id, milestone_id))
            .expect("Milestone not found");

        if milestone.recipient != caller {
            panic!("Only designated recipient can submit milestone");
        }

        if milestone.status != MilestoneStatus::Pending
            && milestone.status != MilestoneStatus::Rejected
        {
            panic!("Milestone is not in submittable state");
        }

        milestone.status = MilestoneStatus::Submitted;
        milestone.submission_ref = submission_ref;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(bounty_id, milestone_id), &milestone);

        // Emit structured event
        env.events().publish(
            (symbol_short!("milestone"), symbol_short!("submitted")),
            (bounty_id, milestone_id, caller),
        );
    }

    /// 5. Community Verification (Approve / Reject) with duplicate vote prevention.
    pub fn verify_milestone(
        env: Env,
        reviewer: Address,
        bounty_id: u64,
        milestone_id: u32,
        decision: VoteDecision,
    ) {
        reviewer.require_auth();

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(bounty_id, milestone_id))
            .expect("Milestone not found");

        if milestone.status != MilestoneStatus::Submitted
            && milestone.status != MilestoneStatus::UnderReview
        {
            panic!("Milestone is not open for review");
        }

        let vote_key = DataKey::Vote(bounty_id, milestone_id, reviewer.clone());
        if env.storage().persistent().has(&vote_key) {
            panic!("Duplicate vote: reviewer has already voted on this milestone");
        }

        // Record vote to prevent double voting
        env.storage().persistent().set(&vote_key, &decision);

        match decision {
            VoteDecision::Approve => {
                milestone.approvals += 1;
            }
            VoteDecision::Reject => {
                milestone.rejections += 1;
            }
        }

        // Check if approval threshold reached
        if milestone.approvals >= milestone.approval_threshold {
            milestone.status = MilestoneStatus::Approved;
            env.events().publish(
                (symbol_short!("milestone"), symbol_short!("approved")),
                (bounty_id, milestone_id, milestone.approvals),
            );
        } else {
            milestone.status = MilestoneStatus::UnderReview;
        }

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(bounty_id, milestone_id), &milestone);
    }

    /// 6. Conditional Payment Release: strictly authorized by the Soroban contract.
    pub fn release_milestone_payment(
        env: Env,
        caller: Address,
        bounty_id: u64,
        milestone_id: u32,
    ) {
        caller.require_auth();

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(bounty_id, milestone_id))
            .expect("Milestone not found");

        // Contract-enforced condition: MUST be Approved
        if milestone.status != MilestoneStatus::Approved {
            panic!("Payment condition not satisfied: milestone has not reached approval threshold");
        }

        let bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        // Disburse funds authoritatively from contract escrow to recipient
        let token_client = token::Client::new(&env, &bounty.token);
        token_client.transfer(
            &env.current_contract_address(),
            &milestone.recipient,
            &milestone.reward_amount,
        );

        milestone.status = MilestoneStatus::Paid;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(bounty_id, milestone_id), &milestone);

        // Emit structured event
        env.events().publish(
            (symbol_short!("milestone"), symbol_short!("paid")),
            (
                bounty_id,
                milestone_id,
                milestone.recipient,
                milestone.reward_amount,
            ),
        );
    }

    /// Query bounty details by ID.
    pub fn get_bounty(env: Env, bounty_id: u64) -> Bounty {
        env.storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found")
    }

    /// Query milestone details.
    pub fn get_milestone(env: Env, bounty_id: u64, milestone_id: u32) -> Milestone {
        env.storage()
            .persistent()
            .get(&DataKey::Milestone(bounty_id, milestone_id))
            .expect("Milestone not found")
    }

    /// Check if reviewer has already voted on milestone.
    pub fn has_voted(env: Env, bounty_id: u64, milestone_id: u32, reviewer: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Vote(bounty_id, milestone_id, reviewer))
    }
}
