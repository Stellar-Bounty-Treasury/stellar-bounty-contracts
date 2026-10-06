#![no_std]
use soroban_sdk::{
    contract, contractimpl, symbol_short, token, Address, Env, Symbol, Vec,
};

pub mod types;

#[cfg(test)]
mod test;

use types::{
    AllocationType, Bounty, BountyStatus, DataKey, Milestone, MilestoneStatus,
    RecipientShare, SettlementConfig, SettlementStatus, VoteDecision,
};

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

        if bounty.status == BountyStatus::Cancelled
            || bounty.status == BountyStatus::Completed
            || bounty.status == BountyStatus::Refunded
        {
            panic!("Cannot fund cancelled, completed, or refunded bounty");
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
        if bounty.status == BountyStatus::Cancelled || bounty.status == BountyStatus::Refunded {
            panic!("Cannot add milestones to cancelled or refunded bounty");
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

    /// 4. Configure multi-recipient settlement router for a milestone.
    pub fn configure_settlement(
        env: Env,
        creator: Address,
        bounty_id: u64,
        milestone_id: u32,
        allocation_type: AllocationType,
        recipients: Vec<RecipientShare>,
    ) -> u32 {
        creator.require_auth();

        let bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != creator {
            panic!("Only bounty creator can configure settlement router");
        }

        let milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(bounty_id, milestone_id))
            .expect("Milestone not found");

        if milestone.status == MilestoneStatus::Paid {
            panic!("Cannot configure settlement for already paid milestone");
        }

        let settlement_key = DataKey::Settlement(bounty_id, milestone_id);
        if let Some(existing) = env.storage().persistent().get::<DataKey, SettlementConfig>(&settlement_key) {
            if existing.is_immutable {
                panic!("Settlement configuration is immutable and locked");
            }
        }

        if recipients.is_empty() {
            panic!("Settlement must contain at least one recipient");
        }

        // Validate recipient uniqueness and allocations
        let mut computed_shares = Vec::new(&env);
        let mut total_allocated: i128 = 0;
        let mut total_bps: u32 = 0;

        for i in 0..recipients.len() {
            let share = recipients.get(i).unwrap();

            // Check duplicate recipients
            for j in 0..i {
                let prev = recipients.get(j).unwrap();
                if prev.recipient == share.recipient {
                    panic!("Duplicate recipient address not allowed");
                }
            }

            match allocation_type {
                AllocationType::Fixed => {
                    if share.amount <= 0 {
                        panic!("Fixed allocation share amount must be positive");
                    }
                    total_allocated += share.amount;
                    computed_shares.push_back(share);
                }
                AllocationType::Percentage => {
                    if share.percentage_bps == 0 || share.percentage_bps > 10_000 {
                        panic!("Percentage share basis points must be between 1 and 10000");
                    }
                    total_bps += share.percentage_bps;
                    computed_shares.push_back(share);
                }
            }
        }

        // Validate allocation totals
        let final_shares = match allocation_type {
            AllocationType::Fixed => {
                if total_allocated != milestone.reward_amount {
                    panic!("Total fixed allocation does not match milestone reward amount");
                }
                computed_shares
            }
            AllocationType::Percentage => {
                if total_bps != 10_000 {
                    panic!("Total percentage basis points must sum exactly to 10000 (100%)");
                }
                // Convert bps to exact amounts and handle rounding residue
                let mut converted_shares = Vec::new(&env);
                let mut sum_calc: i128 = 0;
                let n = computed_shares.len();

                for i in 0..n {
                    let mut item = computed_shares.get(i).unwrap();
                    let calculated_amt = (milestone.reward_amount * item.percentage_bps as i128) / 10_000;
                    item.amount = calculated_amt;
                    sum_calc += calculated_amt;
                    converted_shares.push_back(item);
                }

                // If rounding remainder exists, add to first recipient
                let remainder = milestone.reward_amount - sum_calc;
                if remainder > 0 && !converted_shares.is_empty() {
                    let mut first = converted_shares.get(0).unwrap();
                    first.amount += remainder;
                    converted_shares.set(0, first);
                }

                converted_shares
            }
        };

        let settlement = SettlementConfig {
            settlement_id: milestone_id,
            bounty_id,
            milestone_id,
            allocation_type,
            total_amount: milestone.reward_amount,
            recipients: final_shares,
            status: SettlementStatus::Pending,
            is_immutable: false,
        };

        env.storage().persistent().set(&settlement_key, &settlement);

        // Emit settlement configured event
        env.events().publish(
            (symbol_short!("settle"), symbol_short!("config")),
            (bounty_id, milestone_id, recipients.len()),
        );

        milestone_id
    }

    /// 5. Submit milestone deliverable reference (GitHub PR / commit / CID).
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

        // Allow designated milestone recipient OR designated first settlement recipient
        let settlement_key = DataKey::Settlement(bounty_id, milestone_id);
        let is_authorized_submitter = if milestone.recipient == caller {
            true
        } else if let Some(config) = env.storage().persistent().get::<DataKey, SettlementConfig>(&settlement_key) {
            let mut found = false;
            for i in 0..config.recipients.len() {
                if config.recipients.get(i).unwrap().recipient == caller {
                    found = true;
                    break;
                }
            }
            found
        } else {
            false
        };

        if !is_authorized_submitter {
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

    /// 6. Community Verification (Approve / Reject) with duplicate vote prevention.
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

            // Make settlement config immutable upon approval to prevent front-running
            let settlement_key = DataKey::Settlement(bounty_id, milestone_id);
            if let Some(mut settlement) = env.storage().persistent().get::<DataKey, SettlementConfig>(&settlement_key) {
                settlement.is_immutable = true;
                settlement.status = SettlementStatus::Authorized;
                env.storage().persistent().set(&settlement_key, &settlement);

                env.events().publish(
                    (symbol_short!("settle"), symbol_short!("authoriz")),
                    (bounty_id, milestone_id, settlement.total_amount),
                );
            }

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

    /// 7. Execute Multi-Recipient Settlement through the Settlement Router.
    pub fn execute_settlement(
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

        if milestone.status == MilestoneStatus::Paid {
            panic!("Settlement has already been executed");
        }

        // Contract-enforced condition: MUST be Approved
        if milestone.status != MilestoneStatus::Approved {
            panic!("Payment condition not satisfied: milestone has not reached approval threshold");
        }

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.funded_amount < milestone.reward_amount {
            panic!("Insufficient bounty escrow balance to execute settlement");
        }

        let settlement_key = DataKey::Settlement(bounty_id, milestone_id);
        let token_client = token::Client::new(&env, &bounty.token);

        if let Some(mut settlement) = env.storage().persistent().get::<DataKey, SettlementConfig>(&settlement_key) {
            if settlement.status == SettlementStatus::Settled {
                panic!("Settlement has already been executed");
            }

            settlement.status = SettlementStatus::Executing;
            env.events().publish(
                (symbol_short!("settle"), symbol_short!("started")),
                (bounty_id, milestone_id, settlement.total_amount),
            );

            // Execute atomic transfers to all recipients
            for i in 0..settlement.recipients.len() {
                let share = settlement.recipients.get(i).unwrap();
                token_client.transfer(
                    &env.current_contract_address(),
                    &share.recipient,
                    &share.amount,
                );

                // Emit individual recipient payment event
                env.events().publish(
                    (symbol_short!("settle"), symbol_short!("paid")),
                    (bounty_id, milestone_id, share.recipient, share.amount),
                );
            }

            settlement.status = SettlementStatus::Settled;
            settlement.is_immutable = true;
            env.storage().persistent().set(&settlement_key, &settlement);

            env.events().publish(
                (symbol_short!("settle"), symbol_short!("done")),
                (bounty_id, milestone_id, settlement.total_amount),
            );
        } else {
            // Default single-recipient fallback (Level 2 compatibility)
            token_client.transfer(
                &env.current_contract_address(),
                &milestone.recipient,
                &milestone.reward_amount,
            );

            env.events().publish(
                (symbol_short!("milestone"), symbol_short!("paid")),
                (
                    bounty_id,
                    milestone_id,
                    milestone.recipient.clone(),
                    milestone.reward_amount,
                ),
            );
        }

        // Update bounty escrow balance and milestone status
        bounty.funded_amount -= milestone.reward_amount;
        milestone.status = MilestoneStatus::Paid;

        env.storage()
            .persistent()
            .set(&DataKey::Milestone(bounty_id, milestone_id), &milestone);
        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);
    }

    /// Backward compatibility alias for single/multi-recipient release
    pub fn release_milestone_payment(
        env: Env,
        caller: Address,
        bounty_id: u64,
        milestone_id: u32,
    ) {
        Self::execute_settlement(env, caller, bounty_id, milestone_id);
    }

    /// 8. Refund / Recovery Mechanism: securely return remaining unspent escrow funds.
    pub fn refund_bounty(env: Env, caller: Address, bounty_id: u64) {
        caller.require_auth();

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != caller {
            panic!("Only bounty creator can trigger refund");
        }

        if bounty.status == BountyStatus::Completed || bounty.status == BountyStatus::Refunded {
            panic!("Bounty is already completed or refunded");
        }

        if bounty.funded_amount <= 0 {
            panic!("No remaining escrow balance to refund");
        }

        let refund_amount = bounty.funded_amount;

        // Transfer funds back to creator
        let token_client = token::Client::new(&env, &bounty.token);
        token_client.transfer(
            &env.current_contract_address(),
            &bounty.creator,
            &refund_amount,
        );

        bounty.funded_amount = 0;
        bounty.status = BountyStatus::Refunded;

        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);
        env.storage().persistent().set(&DataKey::Refunded(bounty_id), &true);

        env.events().publish(
            (symbol_short!("refund"), symbol_short!("done")),
            (bounty_id, caller, refund_amount),
        );
    }

    /// 9. Final Bounty Completion: contract-enforced completion once all milestones are paid.
    pub fn complete_bounty(env: Env, caller: Address, bounty_id: u64) {
        caller.require_auth();

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != caller {
            panic!("Only bounty creator can complete bounty");
        }

        if bounty.milestone_count == 0 {
            panic!("Cannot complete bounty with no milestones");
        }

        // Verify all milestones have been Paid
        for m_id in 1..=bounty.milestone_count {
            let milestone: Milestone = env
                .storage()
                .persistent()
                .get(&DataKey::Milestone(bounty_id, m_id))
                .expect("Milestone not found");

            if milestone.status != MilestoneStatus::Paid {
                panic!("All milestones must be paid before marking bounty completed");
            }
        }

        bounty.status = BountyStatus::Completed;
        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);

        env.events().publish(
            (symbol_short!("bounty"), symbol_short!("done")),
            (bounty_id, bounty.milestone_count),
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

    /// Query settlement configuration.
    pub fn get_settlement(env: Env, bounty_id: u64, milestone_id: u32) -> SettlementConfig {
        env.storage()
            .persistent()
            .get(&DataKey::Settlement(bounty_id, milestone_id))
            .expect("Settlement config not found")
    }

    /// Check if settlement configuration exists.
    pub fn has_settlement(env: Env, bounty_id: u64, milestone_id: u32) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Settlement(bounty_id, milestone_id))
    }

    /// Check if bounty was refunded.
    pub fn is_refunded(env: Env, bounty_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Refunded(bounty_id))
    }
}
