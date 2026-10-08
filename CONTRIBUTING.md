# Contributing to Stellar Bounty Contracts

Thank you for contributing to **Stellar Bounty Contracts**! These Soroban smart contracts enforce milestone-based conditional escrow, multi-recipient settlement routing, and trustless refund recovery on Stellar.

---

## Code of Conduct

Please review and adhere to our [Code of Conduct](CODE_OF_CONDUCT.md) in all project interactions.

---

## Prerequisites

* **Rust**: `1.80.0` or later
* **Soroban / Stellar CLI**: latest release
* **Target**: `wasm32-unknown-unknown`
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

---

## Development & Testing Workflow

### 1. Run Unit & Invariant Tests
```bash
cargo test
```
All 16 unit tests must pass cleanly.

### 2. Format & Lint
```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
```

### 3. Build Release WASM
```bash
cargo build --target wasm32-unknown-unknown --release
```

---

## Key Smart Contract Invariants

1. **Safe Integer Arithmetic:** Basis point allocations must sum to exactly $10,000$ ($100.00\%$). Fixed allocations must match total milestone reward.
2. **Reentrancy & Double-Execution Protection:** Milestone payouts and refunds can execute only once. State machine transitions must be irreversible.
3. **Multi-Sig Authorization:** Unapproved settlement routes cannot execute. Approvals are immutable once sealed.

---

## Pull Request Guidelines

1. Fork the repo and create a branch: `feat/your-feature-name` or `fix/issue-description`.
2. Follow Conventional Commits (`feat(contracts): ...`, `fix(settlement): ...`).
3. Ensure `cargo test` and `cargo fmt` pass without warnings.
4. Reference related issues in the PR description (`Closes #...`).

---

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
