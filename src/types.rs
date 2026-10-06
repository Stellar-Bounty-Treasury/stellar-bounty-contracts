use soroban_sdk::{contracttype, Address, Symbol};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BountyStatus {
    Open = 0,
    Funded = 1,
    InVerification = 2,
    Settled = 3,
    Cancelled = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounty {
    pub id: u64,
    pub creator: Address,
    pub title: Symbol,
    pub target_amount: i128,
    pub funded_amount: i128,
    pub status: BountyStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub bounty_id: u64,
    pub index: u32,
    pub description: Symbol,
    pub payout_amount: i128,
    pub approved: bool,
}

#[contracttype]
pub enum DataKey {
    Admin,
    BountyCounter,
    Bounty(u64),
    Milestone(u64, u32),
}
