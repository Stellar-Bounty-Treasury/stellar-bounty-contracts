# Changelog

All notable changes to the **Stellar Bounty Treasury Smart Contracts** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [4.0.0] - 2026-10-29

### Added
- **Formal Invariant Test Suite**: Added `src/test_invariants.rs` with 9 automated invariant verification tests asserting mathematical escrow solvency, 10,000 basis point conservation, reentrancy lockout, fee caps, and RBAC defense.
- **Formal STRIDE Security Threat Model**: Published `docs/SECURITY_THREAT_MODEL.md` analyzing spoofing, tampering, repudiation, information disclosure, denial of service, and elevation of privilege on Soroban.
- **Comprehensive Security Audit Report**: Published `docs/SECURITY_AUDIT_REPORT.md` certifying 39/39 passing tests and zero high or critical vulnerabilities.
- **Disaster Recovery Runbook**: Published `docs/DISASTER_RECOVERY_RUNBOOK.md` detailing operational pause/unpause protocols and state recovery steps.
- **Decentralized Multi-Party Dispute Arbitration**: Added `file_dispute`, `resolve_dispute`, and `has_active_dispute` methods allowing appointed arbitrators to protect contributors and treasuries.
- **10,000 Basis Points Settlement Router**: Strict integer basis point allocation validation enforcing exact 10,000 BPS sum equality for multi-recipient milestone disbursements.
- **Automated Storage Rent TTL Extension**: Implemented `extend_contract_ttl` and `extend_bounty_ttl` to bump persistent ledger thresholds and prevent contract state archival.
- **Milestone Deadline Extensions**: Added `extend_milestone_deadline` enabling bounty creators to grant schedule extensions to contributors.
- **Multi-Token Stellar Asset Contract (SAC) Support**: Upgraded contract to support any Stellar SAC token (native XLM, USDC, EURC) dynamically per bounty escrow.
- **Granular Role-Based Access Control**: Introduced `Role::Admin`, `Role::Operator`, and `Role::Arbitrator` with cryptographic auth guards.
- **Emergency Circuit Breaker**: Added `pause`, `unpause`, and `is_paused` administrative locks halting state mutations during anomalies.
- **Platform Fee Treasury Routing**: Configurable protocol fee capped at 500 basis points (5.0%) routed to governance treasury.
- **Automated Test Runners**: Added `scripts/test_invariants.sh` and `scripts/test_invariants.bat` for cross-platform one-click invariant execution.
- **Soroban WASM Optimizations**: Added `.cargo/config.toml` configuring size-optimization (`opt-level = "z"`) and link stripping.

### Changed
- Refactored typed error handling with comprehensive `ContractError` enum variants.
- Upgraded Soroban SDK dependency to `v22.0.11`.
- Mounted invariant verification into standard `cargo test --lib` test pipeline.

### Fixed
- Fixed integer overflow potential by using `i128` arithmetic throughout token balance logic.
- Prevented duplicate votes on milestones by caching reviewer vote records in persistent storage.
- Fixed premature deadline expiry evaluation by checking current ledger timestamp against explicit milestone bounds.

---

## [3.0.0] - 2026-10-08

### Added
- Level 3 Settlement Router engine for multi-recipient disbursements.
- Fixed and percentage-based allocation types.
- Refund recovery mechanism for expired or abandoned bounties.
- CI/CD workflow testing release WASM compilation.

---

## [2.0.0] - 2026-10-01

### Added
- Milestone lifecycle governance (submission, review, verification quorums).
- Escrow token deposit locking and multi-contributor milestone tracking.

---

## [1.0.0] - 2026-09-15

### Added
- Initial Soroban smart contract for simple bounty creation and release.
