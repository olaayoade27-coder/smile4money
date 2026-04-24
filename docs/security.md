# Threat Model & Security

## Trust Assumptions

| Actor | Trusted for | Not trusted for |
|-------|-------------|-----------------|
| Stellar network | Transaction finality, ledger ordering | — |
| Escrow contract | Correct payout logic (auditable) | — |
| Oracle service | Correct game result reporting | Availability (liveness only) |
| Players | Signing their own transactions | Honesty |
| Admin key | Emergency controls only | Routine operations |

## Threat Scenarios

### Re-initialization Attack
**Threat:** Attacker calls `initialize` a second time to replace the oracle address with a malicious one.  
**Mitigation:** `initialize` checks `env.storage().instance().has(&DataKey::Oracle)` and panics if already set.

### Oracle Substitution
**Threat:** Attacker replaces the oracle address to submit fraudulent results.  
**Mitigation:** Only the admin key can call `update_oracle`. The admin key should be a hardware wallet or multisig.

### Cross-Match Result Injection
**Threat:** A compromised oracle submits a result for match ID `A` using the `game_id` of match `B`, redirecting a payout.  
**Mitigation:** `submit_result` verifies `game_id == m.game_id`. Mismatch returns `Error::GameIdMismatch`.

### Duplicate Game ID
**Threat:** Attacker creates two matches for the same `game_id` and collects double payouts when the oracle submits results for both.  
**Mitigation:** `create_match` stores `DataKey::GameId(game_id)` and rejects duplicates with `Error::AlreadyExists`.

### Self-Match Exploit
**Threat:** A single address creates a match against itself, deposits twice, and receives the full pot back.  
**Mitigation:** `create_match` rejects `player1 == player2` with `Error::InvalidPlayers`.

### Zero-Stake Match
**Threat:** Matches with `stake_amount = 0` waste ledger storage with no economic value.  
**Mitigation:** `create_match` rejects `stake_amount <= 0` with `Error::InvalidAmount`.

### Deposit into Cancelled / Completed Match
**Threat:** Race condition allows a deposit into a match that is being cancelled simultaneously.  
**Mitigation:** `deposit` checks `m.state == MatchState::Pending` and returns `Error::InvalidState` otherwise.

### Double Result Submission
**Threat:** Oracle submits a result twice, potentially triggering a double payout.  
**Mitigation:** `submit_result` checks `m.state == MatchState::Active` and returns `Error::InvalidState` on the second call. The Oracle Contract also deduplicates by `match_id`.

### Ledger Entry Expiry
**Threat:** Persistent storage entries expire after their TTL, causing `MatchNotFound` for active matches.  
**Mitigation:** Every persistent write calls `extend_ttl(key, MATCH_TTL_LEDGERS, MATCH_TTL_LEDGERS)` (~30 days).

### MatchCount Overflow
**Threat:** `MatchCount` wraps silently in release mode, overwriting existing match records.  
**Mitigation:** `checked_add(1)` is used; overflow returns `Error::Overflow`.

### Unauthorized Cancel
**Threat:** A third party cancels a match to grief players.  
**Mitigation:** `cancel_match` requires `caller == player1 || caller == player2` and `caller.require_auth()`.

### Contract Vulnerability Response
**Threat:** A critical bug is discovered post-deployment with no way to stop new activity.  
**Mitigation:** Admin can call `pause()` to block `create_match`, `deposit`, and `submit_result`. Existing matches can still be cancelled to recover funds.

## Admin Key Security

The admin key controls `pause`, `unpause`, and `update_oracle`. Recommendations:

- Use a hardware wallet (Ledger) or a Stellar multisig account as the admin.
- Never store the admin secret key in `.env` or CI secrets for production.
- Rotate the oracle address via `update_oracle` if the oracle service key is compromised — no redeploy needed.

## Known Limitations

- **Oracle liveness**: If the oracle service goes offline, active matches cannot be resolved. Players can cancel pending matches but cannot recover funds from `Active` matches without the oracle. A timeout-based escape hatch is planned for v1.1 (see [Roadmap](roadmap.md)).
- **No fee mechanism**: The contract takes no platform fee. This is intentional for v1.0 but means there is no on-chain revenue to fund oracle operation costs.
- **Single oracle**: There is one trusted oracle address. A decentralised oracle network is a v2.0 consideration.
