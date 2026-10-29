# Disaster Recovery & Incident Response Runbook

Operational procedures for responding to anomalies or compromised credentials.

---

## 1. Emergency Circuit Breaker (Pause Protocol)

If an anomaly is detected, the Admin or Operator immediately triggers emergency pause:

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source <ADMIN_SECRET> \
  -- pause \
  --admin <ADMIN_ADDRESS>
```

All mutating calls (`create_bounty`, `fund_bounty`, `verify_milestone`, `execute_settlement`) will reject with `ContractError::ContractIsPaused`.

---

## 2. Unpause Protocol

Once the incident is resolved:

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  --source <ADMIN_SECRET> \
  -- unpause \
  --admin <ADMIN_ADDRESS>
```
