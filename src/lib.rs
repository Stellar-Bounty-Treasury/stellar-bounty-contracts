#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

pub mod types;

#[cfg(test)]
mod test;

use types::{Bounty, BountyStatus, DataKey};

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

    /// Foundational Bounty Creation module (Level 1).
    /// Stores initial bounty metadata on-chain and returns the generated bounty ID.
    pub fn create_bounty(
        env: Env,
        creator: Address,
        title: Symbol,
        target_amount: i128,
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
            creator,
            title,
            target_amount,
            funded_amount: 0,
            status: BountyStatus::Open,
        };

        env.storage().persistent().set(&DataKey::Bounty(counter), &bounty);
        env.storage().instance().set(&DataKey::BountyCounter, &counter);

        counter
    }

    /// Query bounty details by ID.
    pub fn get_bounty(env: Env, bounty_id: u64) -> Bounty {
        env.storage()
            .persistent()
            .get(&DataKey::Bounty(bounty_id))
            .expect("Bounty not found")
    }
}
