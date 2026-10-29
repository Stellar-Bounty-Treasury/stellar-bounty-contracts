# Soroban Smart Contracts Security Threat Model

A STRIDE-based security analysis of the **Stellar Bounty Treasury** smart contract protocol.

---

## 1. Spoofing & Identity
- **Threat**: Attacker calls `execute_settlement` or `verify_milestone` posing as creator or reviewer.
- **Mitigation**: `initiator.require_auth()` enforces cryptographic Stellar ed25519 signature checks on all mutations.

## 2. Tampering & Invariants
- **Threat**: Attacker creates settlement allocations exceeding milestone reward balance.
- **Mitigation**: Contract asserts strict 10,000 basis points sum equality and non-negative escrow balance invariants.

## 3. Repudiation & Traceability
- **Threat**: Disputed milestone verification without evidence.
- **Mitigation**: Cryptographic event emissions dispatch indexed transaction hashes for all approvals and rejections.

## 4. Information Disclosure
- **Threat**: Unauthorized exposure of sensitive deliverables.
- **Mitigation**: Only deliverable URI references (e.g., GitHub PRs, hashes) are committed on-chain.

## 5. Denial of Service & Storage Rent
- **Threat**: Storage TTL expiration rendering contract state archived or unreachable.
- **Mitigation**: Automated TTL extension helpers (`extend_contract_ttl` and `extend_bounty_ttl`) refresh state thresholds.

## 6. Elevation of Privilege
- **Threat**: Non-arbitrator resolves open dispute.
- **Mitigation**: `require_role(&env, &arbitrator, Role::Arbitrator)` blocks unauthorized verdicts.
