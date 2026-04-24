# Threat Model & Security

This document describes the trust assumptions, attack vectors, and mitigations in smile4money.

## Trust Assumptions

| Actor | Trusted? | Notes |
|---|---|---|
| Stellar network | Yes | Consensus and transaction finality |
| Soroban runtime | Yes | Contract execution environment |
| Escrow contract | Yes | Auditable, open-source |
| Oracle contract | Yes | Auditable, open-source |
| Off-chain oracle service | Partially | Trusted for result accuracy; constrained by contract guards |
| Players | No | Assumed adversarial |
| Chess platforms (Lichess, Chess.com) | Partially | Trusted for game data; oracle verifies terminal state |

The oracle service is the primary trust assumption. It is a centralized component. All other mitigations assume the oracle service may be compromised or malicious.

## Attack Vectors and Mitigations

### 1. Re-initialization attack

**Threat**: An attacker calls `initialize` a second time to overwrite the oracle or admin address with a malicious one.

**Mitigation**: Both `EscrowContract::initialize` and `OracleContract::initialize` check for the existence of their respective admin/oracle keys before writing. A second call panics immediately.

```rust
if env.storage().instance().has(&DataKey::Oracle) {
    panic!("Contract already initialized");
}
```

---

### 2. Unauthorized result submission

**Threat**: A malicious actor calls `EscrowContract::submit_result` with a fabricated winner to steal funds.

**Mitigation**: `submit_result` checks `caller == oracle` and calls `caller.require_auth()`. Any address other than the registered oracle receives `Error::Unauthorized`.

---

### 3. Oracle compromise / key rotation

**Threat**: The oracle service's private key is compromised. The attacker can submit arbitrary results.

**Mitigation**: The admin address in the escrow contract can call `update_oracle(new_oracle)` (planned for v1.1) to rotate the oracle address without redeploying. Until then, a compromised oracle requires a contract redeploy.

**Current limitation**: There is no on-chain oracle rotation function in v1.0. This is a known gap — see [Roadmap](roadmap.md).

---

### 4. Double result submission

**Threat**: The oracle submits a result twice (e.g., due to a bug or retry logic), potentially triggering a double payout.

**Mitigation (Oracle Contract)**: `OracleContract::submit_result` checks `env.storage().persistent().has(&DataKey::Result(match_id))` and returns `Error::AlreadySubmitted` on a duplicate.

**Mitigation (Escrow Contract)**: `EscrowContract::submit_result` checks `m.state == MatchState::Active`. A completed match has `state = Completed`, so a second call returns `Error::InvalidState`.

---

### 5. Deposit into wrong match state

**Threat**: A player deposits into a cancelled or completed match, losing funds with no recourse.

**Mitigation**: `deposit` checks `m.state == MatchState::Pending`. Any other state returns `Error::InvalidState`. Funds are never transferred if the state check fails.

---

### 6. Player1 griefs player2 by cancelling mid-game

**Threat**: Player1 cancels the match after player2 has deposited, forcing a refund and wasting player2's time.

**Mitigation**: Cancellation is only allowed in `Pending` state. Once both players have deposited, the match transitions to `Active` and cannot be cancelled. Player2 can also cancel a `Pending` match if player1 has not deposited.

---

### 7. Self-match exploit

**Threat**: A single address creates a match against itself, deposits twice, and receives the full pot back — wasting ledger storage and potentially gaming any future reward systems.

**Mitigation**: `create_match` validates `player1 != player2`. A self-match returns `Error::InvalidPlayers`. *(Planned for v1.1 — not yet implemented in v1.0.)*

---

### 8. Zero-stake match

**Threat**: A match is created with `stake_amount = 0`, wasting ledger storage with no economic value.

**Mitigation**: `create_match` checks `stake_amount > 0`. A zero or negative stake returns `Error::InvalidAmount`.

---

### 9. Duplicate game ID

**Threat**: The same chess game ID is used in multiple matches. If the oracle submits results for each match ID, the attacker collects multiple payouts from a single game.

**Mitigation**: *(Planned for v1.1)* A `DataKey::GameId(String)` set will track used game IDs. `create_match` will reject duplicates.

---

### 10. Storage expiry (TTL)

**Threat**: A `Match` record expires from persistent storage before the match is resolved. Subsequent calls return `Error::MatchNotFound`, locking funds permanently.

**Mitigation**: Every persistent write calls `extend_ttl` with `MATCH_TTL_LEDGERS = 518_400` (~30 days at 5s/ledger). TTL is refreshed on `create_match`, `deposit`, `submit_result`, and `cancel_match`.

---

### 11. Integer overflow in match counter

**Threat**: `MatchCount` wraps around in release mode (Soroban uses wrapping arithmetic in release builds), causing match ID collisions.

**Mitigation**: `create_match` uses `id.checked_add(1).ok_or(Error::Overflow)?`. An overflow returns `Error::Overflow` rather than silently wrapping.

---

### 12. Contract pause bypass

**Threat**: An attacker calls `create_match`, `deposit`, or `submit_result` while the contract is paused.

**Mitigation**: All three functions check `DataKey::Paused` at entry and return `Error::ContractPaused` if set. The pause flag is set by the admin via `pause()`.

---

### 13. Unauthorized admin operations

**Threat**: A non-admin address calls `pause()` or `unpause()`.

**Mitigation**: Both functions retrieve the stored admin address and call `admin.require_auth()`. Soroban's auth framework rejects any invocation not authorized by the admin's key.

## Known Limitations (v1.0)

- **Centralized oracle**: The oracle service is a single point of failure and trust. A compromised oracle can submit false results.
- **No oracle rotation**: The oracle address cannot be changed post-deploy in v1.0.
- **No timeout refund**: If the oracle goes offline, active matches are stuck. Players cannot claim refunds without oracle intervention.
- **No self-match guard**: `player1 == player2` is not yet rejected (planned for v1.1).
- **No game_id uniqueness check**: The same game ID can be reused across matches (planned for v1.1).

## Audit Checklist

Before mainnet deployment:

- [ ] Independent smart contract audit of escrow and oracle contracts
- [ ] Oracle service key management review (HSM or MPC recommended)
- [ ] Fuzz testing of `create_match`, `deposit`, and `submit_result`
- [ ] Verify TTL values are sufficient for expected match durations
- [ ] Review token contract interactions for reentrancy (Soroban token standard is safe by design, but verify)
- [ ] Confirm `mock_all_auths` is not used in production test paths
