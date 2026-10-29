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

#[allow(dead_code)]
fn acquire_reentrancy_lock(env: &Env) {
    if env.storage().instance().get(&DataKey::Locked).unwrap_or(false) {
        env.panic_with_error(types::ContractError::ReentrancyGuard);
    }
    env.storage().instance().set(&DataKey::Locked, &true);
}

#[allow(dead_code)]
fn release_reentrancy_lock(env: &Env) {
    env.storage().instance().set(&DataKey::Locked, &false);
}

fn require_not_paused(env: &Env) {
    if env.storage().instance().get(&DataKey::Paused).unwrap_or(false) {
        env.panic_with_error(types::ContractError::ContractPaused);
    }
}

#[contract]
pub struct BountyTreasuryContract;

#[contractimpl]
impl BountyTreasuryContract {
    /// Initialize the Bounty Treasury contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            env.panic_with_error(types::ContractError::AlreadyInitialized);
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
            .unwrap_or_else(|| env.panic_with_error(types::ContractError::NotInitialized))
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
        require_not_paused(&env);
        creator.require_auth();

        if target_amount <= 0 {
            env.panic_with_error(types::ContractError::InvalidTargetAmount);
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
            expiry_timestamp: 0,
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
        require_not_paused(&env);
        funder.require_auth();

        if amount <= 0 {
            env.panic_with_error(types::ContractError::InvalidFundingAmount);
        }
        if amount < 1_0000000i128 {
            env.panic_with_error(types::ContractError::AmountTooSmall);
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
        require_not_paused(&env);
        creator.require_auth();

        if reward_amount <= 0 {
            env.panic_with_error(types::ContractError::InvalidFundingAmount);
        }
        if approval_threshold == 0 {
            env.panic_with_error(types::ContractError::InvalidThreshold);
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

        if bounty.milestone_count >= 25 {
            env.panic_with_error(types::ContractError::InvalidMilestoneCount);
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
            deadline: 0,
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

        // Verify deadline has not elapsed if configured
        if milestone.deadline > 0 && env.ledger().timestamp() > milestone.deadline {
            env.panic_with_error(types::ContractError::DeadlineExpired);
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

        // Deduct platform protocol fee if configured
        let (fee_bps, fee_recipient) = Self::get_platform_fee(env.clone());
        let fee_amount = if fee_bps > 0 {
            (milestone.reward_amount * fee_bps as i128) / 10_000
        } else {
            0
        };

        if fee_amount > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &fee_recipient,
                &fee_amount,
            );
            env.events().publish(
                (symbol_short!("fee"), symbol_short!("collected")),
                (bounty_id, milestone_id, fee_amount, fee_recipient),
            );
        }

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
            // Default single-recipient direct transfer fallback
            let net_payout = milestone.reward_amount - fee_amount;
            token_client.transfer(
                &env.current_contract_address(),
                &milestone.recipient,
                &net_payout,
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


    /// Check whether the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
    }

    /// Emergency circuit breaker to halt state-mutating operations (admin only).
    pub fn pause(env: Env, admin: Address) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_or_else(|| {
            env.panic_with_error(types::ContractError::NotInitialized)
        });
        if admin != current_admin {
            env.panic_with_error(types::ContractError::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((symbol_short!("contract"), symbol_short!("paused")), admin);
    }

    /// Lift emergency pause (admin only).
    pub fn unpause(env: Env, admin: Address) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_or_else(|| {
            env.panic_with_error(types::ContractError::NotInitialized)
        });
        if admin != current_admin {
            env.panic_with_error(types::ContractError::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish((symbol_short!("contract"), symbol_short!("unpaused")), admin);
    }


    /// Grant role to an account (admin only).
    pub fn grant_role(env: Env, admin: Address, account: Address, role: types::Role) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_or_else(|| {
            env.panic_with_error(types::ContractError::NotInitialized)
        });
        if admin != current_admin {
            env.panic_with_error(types::ContractError::Unauthorized);
        }
        env.storage().persistent().set(&DataKey::Role(account.clone(), role as u32), &true);
        env.events().publish((symbol_short!("role"), symbol_short!("granted")), (account, role as u32));
    }

    /// Revoke role from an account (admin only).
    pub fn revoke_role(env: Env, admin: Address, account: Address, role: types::Role) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_or_else(|| {
            env.panic_with_error(types::ContractError::NotInitialized)
        });
        if admin != current_admin {
            env.panic_with_error(types::ContractError::Unauthorized);
        }
        env.storage().persistent().set(&DataKey::Role(account.clone(), role as u32), &false);
        env.events().publish((symbol_short!("role"), symbol_short!("revoked")), (account, role as u32));
    }


    /// Check whether an account holds a given role.
    pub fn has_role(env: Env, account: Address, role: types::Role) -> bool {
        env.storage().persistent().get(&DataKey::Role(account, role as u32)).unwrap_or(false)
    }


    /// Configure platform fee basis points and treasury address (admin only, capped at 500 BPS / 5%).
    pub fn set_platform_fee(env: Env, admin: Address, fee_bps: u32, fee_recipient: Address) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap_or_else(|| {
            env.panic_with_error(types::ContractError::NotInitialized)
        });
        if admin != current_admin {
            env.panic_with_error(types::ContractError::Unauthorized);
        }
        if fee_bps > 500 {
            env.panic_with_error(types::ContractError::InvalidFeeBps);
        }
        env.storage().instance().set(&DataKey::PlatformFeeBps, &fee_bps);
        env.storage().instance().set(&DataKey::FeeRecipient, &fee_recipient);
        env.events().publish((symbol_short!("fee"), symbol_short!("updated")), (fee_bps, fee_recipient));
    }


    /// Query current platform fee basis points and fee recipient address.
    pub fn get_platform_fee(env: Env) -> (u32, Address) {
        let fee_bps = env.storage().instance().get(&DataKey::PlatformFeeBps).unwrap_or(0u32);
        let recipient = env.storage().instance().get(&DataKey::FeeRecipient).unwrap_or_else(|| {
            env.storage().instance().get(&DataKey::Admin).unwrap_or_else(|| {
                env.panic_with_error(types::ContractError::NotInitialized)
            })
        });
        (fee_bps, recipient)
    }


    /// Extend or set milestone deliverable deadline (creator only).
    pub fn extend_milestone_deadline(
        env: Env,
        creator: Address,
        bounty_id: u64,
        milestone_id: u32,
        new_deadline: u64,
    ) {
        require_not_paused(&env);
        creator.require_auth();

        let bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != creator {
            env.panic_with_error(types::ContractError::Unauthorized);
        }

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(bounty_id, milestone_id))
            .expect("Milestone not found");

        if milestone.status == MilestoneStatus::Paid {
            env.panic_with_error(types::ContractError::AlreadySettled);
        }

        milestone.deadline = new_deadline;
        env.storage()
            .persistent()
            .set(&DataKey::Milestone(bounty_id, milestone_id), &milestone);

        env.events().publish(
            (symbol_short!("milestone"), symbol_short!("extended")),
            (bounty_id, milestone_id, new_deadline),
        );
    }


    /// Cancel an unpaid milestone and return reward balance back to bounty pool (creator only).
    pub fn cancel_milestone(
        env: Env,
        creator: Address,
        bounty_id: u64,
        milestone_id: u32,
    ) {
        require_not_paused(&env);
        creator.require_auth();

        let bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != creator {
            env.panic_with_error(types::ContractError::Unauthorized);
        }

        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(bounty_id, milestone_id))
            .expect("Milestone not found");

        if milestone.status == MilestoneStatus::Paid {
            env.panic_with_error(types::ContractError::AlreadySettled);
        }

        milestone.status = MilestoneStatus::Rejected;
        env.storage()
            .persistent()
            .set(&DataKey::Milestone(bounty_id, milestone_id), &milestone);

        env.events().publish(
            (symbol_short!("milestone"), symbol_short!("cancelled")),
            (bounty_id, milestone_id),
        );
    }


    /// Query whether a bounty escrow is eligible for refund recovery.
    pub fn can_refund(env: Env, bounty_id: u64) -> bool {
        if let Some(bounty) = env.storage().persistent().get::<DataKey, Bounty>(&DataKey::Bounty(bounty_id)) {
            bounty.funded_amount > 0 && bounty.status != BountyStatus::Completed && bounty.status != BountyStatus::Refunded
        } else {
            false
        }
    }


    /// Set or extend overall bounty escrow expiration timestamp (creator only).
    pub fn update_bounty_expiry(
        env: Env,
        creator: Address,
        bounty_id: u64,
        expiry: u64,
    ) {
        require_not_paused(&env);
        creator.require_auth();

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != creator {
            env.panic_with_error(types::ContractError::Unauthorized);
        }

        bounty.expiry_timestamp = expiry;
        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);

        env.events().publish(
            (symbol_short!("bounty"), symbol_short!("expiry")),
            (bounty_id, expiry),
        );
    }


    /// Update bounty title and external metadata CID reference (creator only).
    pub fn update_bounty_metadata(
        env: Env,
        creator: Address,
        bounty_id: u64,
        new_title: Symbol,
    ) {
        require_not_paused(&env);
        creator.require_auth();

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != creator {
            env.panic_with_error(types::ContractError::Unauthorized);
        }

        bounty.title = new_title;
        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);

        env.events().publish(
            (symbol_short!("bounty"), symbol_short!("updated")),
            (bounty_id, new_title),
        );
    }


    /// Query current funded escrow balance for a bounty.
    pub fn get_bounty_escrow_balance(env: Env, bounty_id: u64) -> i128 {
        let bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");
        bounty.funded_amount
    }


    /// Create multiple milestones atomically in a single transaction (creator only).
    pub fn create_milestones_batch(
        env: Env,
        creator: Address,
        bounty_id: u64,
        milestones: Vec<Milestone>,
    ) -> u32 {
        require_not_paused(&env);
        creator.require_auth();

        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found");

        if bounty.creator != creator {
            env.panic_with_error(types::ContractError::Unauthorized);
        }

        let count = milestones.len();
        if count == 0 || count > 10 {
            env.panic_with_error(types::ContractError::InvalidMilestoneCount);
        }

        for i in 0..count {
            let item = milestones.get(i).unwrap();
            bounty.milestone_count += 1;
            let m_id = bounty.milestone_count;

            let mut m = item;
            m.id = m_id;
            m.bounty_id = bounty_id;
            m.status = MilestoneStatus::Pending;
            m.approvals = 0;
            m.rejections = 0;

            env.storage()
                .persistent()
                .set(&DataKey::Milestone(bounty_id, m_id), &m);
        }

        env.storage().persistent().set(&DataKey::Bounty(bounty_id), &bounty);
        bounty.milestone_count
    }

    /// Check if bounty was refunded.
    pub fn is_refunded(env: Env, bounty_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Refunded(bounty_id))
    }
}
