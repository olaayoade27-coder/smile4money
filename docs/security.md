# Threat Model & Security

## Trust Assumptions

| Actor | Trusted for | Not trusted for |
|-------|-------------|-----------------|
| Stellar network | Correct execution of Soroban contracts | — |
| Oracle service | Accurate game result reporting | Custody of player funds |
| Admin key | Pause/unpause, oracle rotation | Accessing escrowed funds |
| Players | Their own key management | Each other |

No single party can unilaterally steal funds. The escrow contract enforces all payout rules on-chain.

## Threat Model

### Re-initialization Attack

**Threat**: An attacker calls `initialize` a second time to overwrite the oracle or admin address.

**Mitigation**: Both contracts check `env.storage().instance().has(&DataKey::Oracle/Admin)` before writing. A second call panics immediately.

### Malicious Oracle Substitution

**Threat**: An attacker replaces the oracle address to redirect payouts.

**Mitigation**: The oracle address is set at initialization and can only be rotated by the admin via `update_oracle`. The admin key is separate from the oracle key.

### Unauthorized Result Submission

**Threat**: A non-oracle address calls `submit_result` on the escrow contract.

**Mitigation**: `submit_result` compares `caller` against the stored `DataKey::Oracle` address and returns `Error::Unauthorized` on mismatch, before any state change.

### Double Result Submission

**Threat**: The oracle submits a result twice, potentially changing the winner.

**Mitigation**: The escrow contract checks `m.state == Active` before processing. Once set to `Completed`, any further `submit_result` call returns `Error::InvalidState`. The oracle contract additionally rejects duplicate submissions with `Error::AlreadySubmitted`.

### Cross-Match Result Injection (GameIdMismatch)

**Threat**: A compromised oracle submits a result for the correct `match_id` but with a `game_id` belonging to a different game, redirecting a payout to the wrong winner.

**Mitigation**: `submit_result` on the escrow contract requires a `game_id` parameter and compares it against the `game_id` stored in the match record at creation time. A mismatch returns `Error::GameIdMismatch` before any state change or token transfer occurs.

### Duplicate Game ID Exploit (DuplicateGameId)

**Threat**: An attacker creates multiple matches referencing the same chess `game_id`. If the oracle submits a result for that game, all duplicate matches could be paid out, draining the contract.

**Mitigation**: `create_match` tracks every accepted `game_id` in persistent storage under `DataKey::GameId(game_id)`. A second `create_match` call with the same `game_id` returns `Error::DuplicateGameId` immediately, before any match record is written.

### Deposit into Inactive Match

**Threat**: A player deposits into a cancelled or completed match, locking funds.

**Mitigation**: `deposit` checks `m.state == Pending` and returns `Error::InvalidState` for any other state.

### Zero-Stake Match

**Threat**: A match is created with `stake_amount = 0`, wasting ledger storage.

**Mitigation**: `create_match` returns `Error::InvalidAmount` if `stake_amount <= 0`.

### Self-Match

**Threat**: A single address creates a match against itself to manipulate state or waste resources.

**Mitigation**: `create_match` checks `player1 != player2` and returns `Error::InvalidPlayers` on violation.

### Storage Expiry

**Threat**: A persistent `Match` entry expires mid-game, causing `MatchNotFound` errors.

**Mitigation**: Every persistent write calls `extend_ttl` with `MATCH_TTL_LEDGERS` (518,400 ledgers, ~30 days). TTL is refreshed on deposit, result submission, and cancellation.

### Contract Vulnerability Response

**Threat**: A critical bug is discovered post-deployment with no way to stop ongoing damage.

**Mitigation**: The admin can call `pause()` to block `create_match`, `deposit`, and `submit_result`. Existing matches can still be cancelled to recover funds. `unpause()` restores normal operation.

### Integer Overflow in Match Counter

**Threat**: `MatchCount` wraps silently in release mode, reusing match IDs.

**Mitigation**: `create_match` uses `id.checked_add(1).ok_or(Error::Overflow)?`.

## Access Control Summary

| Function | Who can call |
|----------|-------------|
| `initialize` | Anyone (once only) |
| `create_match` | `player1` (requires auth) |
| `deposit` | `player1` or `player2` (requires auth) |
| `cancel_match` | `player1` or `player2` (requires auth) |
| `submit_result` | Registered oracle address only |
| `pause` / `unpause` | Admin only |
| `get_match` / `is_funded` / `get_escrow_balance` | Anyone (read-only) |

## Known Limitations

- The oracle service is a centralised component. A compromised oracle key can submit incorrect results. Key rotation via `update_oracle` mitigates this without redeployment.
- There is no timeout mechanism to auto-cancel a match if the oracle never submits a result. Players must manually cancel a pending match to recover funds.
- The admin `pause` function does not block `cancel_match`, so players can always recover deposits even when the contract is paused.

## Reporting Vulnerabilities

Open a GitHub issue with the label `security`. For critical vulnerabilities, contact the maintainers directly before public disclosure.
