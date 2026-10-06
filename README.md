# 📜 Stellar Bounty Treasury — Smart Contracts

Soroban smart contract foundation and specifications for **Stellar Bounty Treasury**, a community-funded bounty payment platform on the Stellar network.

---

## 📌 What It Does

At **Level 1 (White Belt)**, this repository establishes the formal Soroban contract architecture, build system, types, storage schemas, and unit testing harness.

The complete on-chain escrow, milestone verification, and conditional settlement systems are planned for future levels. In Level 1, we lay down the strict data structures, storage keys, authorization patterns, and unit tests to ensure that Level 2 and Level 3 build on an immutable and audited foundation.

---

## 🏛️ Planned Contract Architecture

The on-chain smart contract is architected into modular components:

```text
Bounty Treasury Contract
├── 1. Creation           (On-chain bounty specification, target amount, creator)
├── 2. Funding            (Escrow vault, contribution ledger, fund lockup)
├── 3. Milestones         (Deliverable milestones, payout allocations)
├── 4. Verification       (Attestation, oracle checks, reviewer signatures)
├── 5. Conditional Release(Time-locks, milestone unlock criteria, quorum verification)
└── 6. Settlement         (Automated token distribution to maintainers/contributors)
```

### Module Breakdown

| Module | Purpose | Status |
| :--- | :--- | :--- |
| **Creation** | Registers bounty ID, title symbol, target XLM amount, and creator auth | **Level 1 (Foundation Active)** |
| **Funding** | Manages escrow vault address and incoming XLM transfers | *Level 2 Scope* |
| **Milestones** | Stores breakdown of milestones and percentage distributions | *Level 2 Scope* |
| **Verification** | Validates milestone proofs from contributors and arbiters | *Level 2 Scope* |
| **Conditional Release** | Disburses locked funds upon satisfaction of milestone criteria | *Level 2 / Level 3 Scope* |
| **Settlement** | Multi-recipient payouts and automated refund handling | *Level 3 Scope* |

---

## 🚀 How to Run It

### Prerequisites

* Rust 1.80+ (`rustup target add wasm32-unknown-unknown`)
* Soroban SDK `v22.0.11`

### Running Unit Tests

Execute the unit test suite locally:

```bash
cargo test
```

### Building the WASM Artifact

Compile the contract to WebAssembly target:

```bash
cargo build --target wasm32-unknown-unknown --release
```

The resulting compiled WASM will be located at:
`target/wasm32-unknown-unknown/release/stellar_bounty_contracts.wasm`

---

## ⚙️ Required Environment Variables

For local contract compilation and testing, standard Cargo tooling is used. No `.env` secrets or private keys are required or committed:

* `STELLAR_NETWORK=testnet` (for future contract deployment via Stellar CLI)
* `SOROBAN_RPC_URL=https://soroban-testnet.stellar.org` (for future deployment)

---

## 👛 How to Connect a Stellar Testnet Wallet

For smart contracts:
1. Generate an identity using Stellar CLI or Freighter wallet.
2. In Soroban contracts, callers authenticate invocations using `Address::require_auth()`.
3. In Level 1 tests, simulated test accounts and auth mocks are provisioned via `env.mock_all_auths()` and `Address::generate(&env)`.

---

## 📝 How to Create a Bounty

At the contract layer, bounties are created via the contract's public method:

```rust
pub fn create_bounty(
    env: Env,
    creator: Address,
    title: Symbol,
    target_amount: i128,
) -> u64
```

* Requires authorization from the `creator` address.
* Enforces positive `target_amount`.
* Increments internal persistent bounty counter and saves the `Bounty` record to storage.

---

## 💸 How to Fund a Bounty

At Level 1, funding is executed peer-to-peer on Stellar Testnet and validated via Horizon. For Level 2, the contract funding module will accept Soroban token transfers:
1. Contributor invokes `fund_bounty(env, contributor: Address, bounty_id: u64, amount: i128)`.
2. The contract invokes the Stellar Asset Contract (SAC) to transfer tokens to the contract escrow vault.
3. The contract updates the persistent `funded_amount` state.

---

## 🔍 How to Verify a Transaction

In Soroban contracts:
1. Transactions generate on-chain events via `env.events().publish(...)`.
2. Unit tests verify state changes using contract clients (`client.get_bounty(&bounty_id)`).
3. On Testnet, contract invocations are verified via Soroban RPC `getTransaction` queries.

---

## 🔄 How the Repository Will Evolve in Levels 2 and 3

```text
LEVEL 1 (Current)
  Create Bounty Foundation ➔ Struct & Storage Key Validation ➔ WASM Build & Testing Harness

LEVEL 2 (Next Milestone)
  On-chain Escrow Vault ➔ Milestone Submissions ➔ Community/Arbiter Verification ➔ Conditional Release

LEVEL 3 (Complete Bounty Treasury)
  Multi-recipient Distribution ➔ Realtime Contract Event Streaming ➔ Autonomous Treasury Governance
```

---

## 🧪 Test Suite Coverage

The Level 1 contract test suite covers:
1. `test_initialize_and_get_admin`: Verifies contract initialization and storage of admin identity.
2. `test_double_initialization_fails`: Asserts panic when attempting re-initialization.
3. `test_create_and_get_bounty`: Verifies creation of bounty with creator authorization, target amount, and initial open status.
4. `test_multiple_bounties_sequential_ids`: Ensures sequential auto-incrementing ID assignment across multiple bounties.
5. `test_create_bounty_zero_target_fails`: Asserts rejection of zero target amounts.
6. `test_create_bounty_negative_target_fails`: Asserts rejection of negative amounts.
7. `test_get_nonexistent_bounty_fails`: Asserts panic when querying unindexed bounty IDs.

---

## 📄 License

MIT
