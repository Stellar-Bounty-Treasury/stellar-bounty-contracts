use soroban_sdk::{contracterror, contracttype, Address, Symbol, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidTargetAmount = 4,
    InvalidFundingAmount = 5,
    BountyNotFound = 6,
    MilestoneNotFound = 7,
    BountyNotOpen = 8,
    MilestoneNotApproved = 9,
    AlreadySettled = 10,
    ContractPaused = 11,
    ReentrancyGuard = 12,
    InvalidFeeBps = 13,
    InvalidThreshold = 14,
    AlreadyVoted = 15,
    MilestoneExceedsFunding = 16,
    AllocationMismatch = 17,
    DuplicateRecipient = 18,
    DisputeAlreadyActive = 19,
    DisputeNotFound = 20,
    DeadlineNotPassed = 21,
    DeadlineExpired = 22,
    InsufficientEscrow = 23,
    MilestonesPending = 24,
    RoleRequired = 25,
    InvalidMilestoneCount = 26,
    AmountTooSmall = 27,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BountyStatus {
    Draft = 0,
    Open = 1,
    Funded = 2,
    InProgress = 3,
    MilestoneReview = 4,
    MilestoneApproved = 5,
    MilestonePaid = 6,
    Completed = 7,
    Cancelled = 8,
    Refunded = 9,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    Pending = 0,
    Submitted = 1,
    UnderReview = 2,
    Approved = 3,
    Paid = 4,
    Rejected = 5,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoteDecision {
    Approve = 1,
    Reject = 2,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationType {
    Fixed = 1,
    Percentage = 2,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettlementStatus {
    None = 0,
    Pending = 1,
    Authorized = 2,
    Executing = 3,
    Settled = 4,
    Failed = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipientShare {
    pub recipient: Address,
    pub amount: i128,
    pub percentage_bps: u32,
    pub label: Symbol,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementConfig {
    pub settlement_id: u32,
    pub bounty_id: u64,
    pub milestone_id: u32,
    pub allocation_type: AllocationType,
    pub total_amount: i128,
    pub recipients: Vec<RecipientShare>,
    pub status: SettlementStatus,
    pub is_immutable: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bounty {
    pub id: u64,
    pub creator: Address,
    pub title: Symbol,
    pub target_amount: i128,
    pub funded_amount: i128,
    pub token: Address,
    pub milestone_count: u32,
    pub status: BountyStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u32,
    pub bounty_id: u64,
    pub description: Symbol,
    pub reward_amount: i128,
    pub recipient: Address,
    pub status: MilestoneStatus,
    pub approval_threshold: u32,
    pub approvals: u32,
    pub rejections: u32,
    pub submission_ref: Symbol,
}

#[contracttype]
pub enum DataKey {
    Admin,
    BountyCounter,
    Bounty(u64),
    Milestone(u64, u32),
    Vote(u64, u32, Address),
    Settlement(u64, u32),
    Refunded(u64),
}
