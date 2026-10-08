# Security Policy — Stellar Bounty Contracts

The Stellar Bounty Treasury team prioritizes the cryptographic and financial safety of our Soroban escrow and settlement smart contracts.

---

## Supported Versions

Only contract code on the `main` branch is supported for security updates.

| Contract | Supported |
| :--- | :--- |
| `src/lib.rs` (`main`) | :white_check_mark: |
| Older releases | :x: |

---

## Reporting a Vulnerability

**Do not report vulnerabilities via public GitHub issues.**

Please disclose security vulnerabilities responsibly:
1. Email: **`security@stellar-bounty-treasury.org`**
2. GitHub Private Advisory: [Report a vulnerability](https://github.com/Stellar-Bounty-Treasury/stellar-bounty-contracts/security/advisories)

### Response SLA
* **Initial Response:** Within 24 hours.
* **Triage & Classification:** Within 48 hours.
* **Remediation:** Coordinated private patch within 7 days.

---

## Smart Contract Invariants & Threat Model

* **Integer Math & Dust Rounding:** Allocation calculations enforce safe integer arithmetic. Overflows, underflows, and rounding remainder loss are strictly prevented.
* **Double-Spending Prevention:** Milestones cannot be claimed or settled twice (`CannotExecuteTwice`).
* **Unauthorized Refund Prevention:** Escrow deposits can only be refunded to the authentic bounty creator following expiry or consensus criteria.
* **Multi-Recipient Settlement Integrity:** Settlement routes cannot execute without required multi-signature threshold approvals.
