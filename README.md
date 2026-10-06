# 🏦 Stellar Bounty Treasury — Soroban Smart Contracts

[![Soroban](https://img.shields.io/badge/Soroban-Rust%20SDK-7c3aed.svg)](https://soroban.stellar.org)
[![Stellar Testnet](https://img.shields.io/badge/Stellar-Testnet-blue.svg)](https://stellar.org)
[![WASM Target](https://img.shields.io/badge/WASM-Compiled%20(37KB)-orange.svg)](target/wasm32-unknown-unknown/release/stellar_bounty_contracts.wasm)
[![CI/CD](https://img.shields.io/badge/CI%2FCD-Passing-brightgreen.svg)](https://github.com/Stellar-Bounty-Treasury/stellar-bounty-contracts/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Soroban smart contracts power the decentralized core of **Stellar Bounty Treasury**.

The contracts enforce trustless milestone-based conditional escrow, community governance approval quorums, a multi-recipient programmable Settlement Router, atomicity guarantees, refund mechanisms, and lifecycle events on the Stellar blockchain.

---

## 🎬 Product Demonstration

![Stellar Bounty Treasury Walkthrough](docs/evidence/level3_demo.gif)

* **Direct Video Links**: [High-Definition MP4](docs/evidence/level3_demo.mp4) • [WebM Video](docs/evidence/level3_demo.webm)
* **Live Web Application**: [https://stellar-bounty-treasury-2676.netlify.app](https://stellar-bounty-treasury-2676.netlify.app)

---

## 📍 Deployed Contract Details

| Property | Value |
|:---------|:------|
| **Network** | Stellar Testnet |
| **Contract ID** | `CADMWQPCCQP27UHQU4JG3C6V5I3UFNNC4DVOMSK2GUJFA6Q2PNW36S52` |
| **WASM Binary Size** | 37.2 KB (`stellar_bounty_contracts.wasm`) |
| **Stellar.Expert Explorer** | [View Contract on Stellar.Expert](https://stellar.expert/explorer/testnet/contract/CADMWQPCCQP27UHQU4JG3C6V5I3UFNNC4DVOMSK2GUJFA6Q2PNW36S52) |

---

## 🏛️ Architecture & Contract Responsibilities

```text
               ┌───────────────────────────────┐
               │    Bounty Treasury Escrow     │
               └───────────────┬───────────────┘
                               │
            ┌──────────────────┴──────────────────┐
            ▼                                     ▼
┌───────────────────────────────┐   ┌───────────────────────────────┐
│     Milestone Governance      │   │       Settlement Router       │
│  • Work Submission Evidence   │   │  • Multi-Recipient Routing    │
│  • Quorum Verification Votes  │   │  • Fixed & Percentage Splits  │
│  • Threshold State Machine    │   │  • Atomic Token Transfers     │
└───────────────────────────────┘   └───────────────┬───────────────┘
                                                    │
                               ┌────────────────────┴────────────────────┐
                               ▼                    ▼                    ▼
                        ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
                        │ Recipient A │      │ Recipient B │      │ Recipient C │
                        │  (e.g. 70%) │      │  (e.g. 20%) │      │  (e.g. 10%) │
                        └─────────────┘      └─────────────┘      └─────────────┘
```

---

## 🔑 Core Features & Guarantees

### 1. 🔀 Programmable Settlement Router
* **Multi-Recipient Distribution**: Supports splitting milestone rewards among arbitrary recipients.
* **Two Settlement Models**:
  * **Fixed Amounts (`AllocationType::Fixed = 1`)**: Payout amounts must sum exactly to the approved milestone reward.
  * **Percentage Splits (`AllocationType::Percentage = 2`)**: Allocation basis points (`percentage_bps`) must sum exactly to `10,000` (100.00%). Excess or deficiency triggers an immediate contract panic.
* **Arithmetic Precision & Safe Math**: Calculated using 128-bit integers (`i128`), ensuring zero overflow, truncation, or rounding exploitation.

### 2. 🔒 Immutability Guarantee
* Once a milestone reaches the `Approved` state, its settlement rules become permanently immutable.
* Bounty creators cannot modify recipient addresses or allocations after community approval, eliminating frontrunning and rug-pull vectors.

### 3. ⚛️ Atomic Execution & State Machine
* All recipient transfers execute within a single Soroban atomic transaction. If any transfer fails, the entire transaction reverts cleanly.
* Explicit settlement states:
  $$\text{Pending} \longrightarrow \text{Authorized} \longrightarrow \text{Executing} \longrightarrow \text{Settled}$$
* **Replay Protection**: A settlement cannot be executed twice (`AlreadySettled` assertion).

### 4. 💸 Escrow Refund / Recovery Mechanism
* Contributor protection: If a bounty is cancelled before milestones are executed, the bounty creator can trigger `refund_bounty(bounty_id)`.
* Verifies that unspent funds are strictly returned and marks the escrow permanently refunded.

### 5. 🏁 Contract-Enforced Bounty Completion
* `complete_bounty(bounty_id)`: Verifies on-chain that all registered milestones are resolved (`Paid` or settled) before transitioning the bounty to `Completed`.

### 6. 📢 Structured Event System
Emits typed Soroban contract events for complete off-chain synchronization:
* `(Symbol::new(&env, "bounty"), Symbol::new(&env, "created"))`
* `(Symbol::new(&env, "bounty"), Symbol::new(&env, "funded"))`
* `(Symbol::new(&env, "milestone"), Symbol::new(&env, "created"))`
* `(Symbol::new(&env, "milestone"), Symbol::new(&env, "submitted"))`
* `(Symbol::new(&env, "milestone"), Symbol::new(&env, "verified"))`
* `(Symbol::new(&env, "milestone"), Symbol::new(&env, "approved"))`
* `(Symbol::new(&env, "settle"), Symbol::new(&env, "config"))`
* `(Symbol::new(&env, "settle"), Symbol::new(&env, "authoriz"))`
* `(Symbol::new(&env, "settle"), Symbol::new(&env, "started"))`
* `(Symbol::new(&env, "settle"), Symbol::new(&env, "paid"))`
* `(Symbol::new(&env, "settle"), Symbol::new(&env, "done"))`
* `(Symbol::new(&env, "refund"), Symbol::new(&env, "done"))`
* `(Symbol::new(&env, "bounty"), Symbol::new(&env, "done"))`

---

## 🛠️ Public Contract Interface

```rust
pub fn create_bounty(env: Env, creator: Address, token: Address, target_amount: i128) -> u32;
pub fn fund_bounty(env: Env, contributor: Address, bounty_id: u32, amount: i128);
pub fn add_milestone(env: Env, caller: Address, bounty_id: u32, reward_amount: i128, recipient: Address, approval_threshold: u32) -> u32;
pub fn submit_milestone(env: Env, submitter: Address, bounty_id: u32, milestone_id: u32);
pub fn verify_milestone(env: Env, voter: Address, bounty_id: u32, milestone_id: u32, approve: bool);
pub fn configure_settlement(env: Env, caller: Address, bounty_id: u32, milestone_id: u32, allocation_type: AllocationType, recipients: Vec<RecipientShare>);
pub fn execute_settlement(env: Env, caller: Address, bounty_id: u32, milestone_id: u32);
pub fn refund_bounty(env: Env, caller: Address, bounty_id: u32);
pub fn complete_bounty(env: Env, caller: Address, bounty_id: u32);
```

---

## 🧪 Testing Suite

Run the complete Soroban SDK test suite:

```bash
cargo test
```

16 comprehensive Rust tests passing cleanly:
* `test_bounty_creation_and_funding`
* `test_milestone_lifecycle_and_approval`
* `test_milestone_rejection`
* `test_double_voting_prevention`
* `test_settlement_router_percentage_success`
* `test_settlement_router_fixed_success`
* `test_settlement_router_invalid_percentage_fails`
* `test_settlement_router_invalid_fixed_sum_fails`
* `test_settlement_router_empty_recipients_fails`
* `test_settlement_cannot_execute_unapproved`
* `test_settlement_cannot_execute_twice`
* `test_settlement_immutable_after_approval`
* `test_refund_bounty_success`
* `test_refund_bounty_unauthorized_fails`
* `test_refund_cannot_execute_twice`
* `test_complete_bounty_success`

---

## 📦 Compilation & Build

```bash
# Build release WASM binary
cargo build --target wasm32-unknown-unknown --release

# Output binary located at:
# target/wasm32-unknown-unknown/release/stellar_bounty_contracts.wasm
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
