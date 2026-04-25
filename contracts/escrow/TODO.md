# Escrow Contract — Completed Work

All items from GitHub issue #111 have been resolved.

## Fixes Applied

- [x] **Double-initialize guard** — `initialize` checks `DataKey::Oracle` existence and panics if already set
- [x] **Zero-stake guard** — `create_match` returns `Error::InvalidAmount` if `stake_amount <= 0`
- [x] **Self-match guard** — `create_match` returns `Error::InvalidPlayers` if `player1 == player2`
- [x] **Duplicate game_id guard** — `create_match` tracks used `game_id` values in `DataKey::GameId(String)` and returns `Error::DuplicateGameId` on collision
- [x] **game_id verification in submit_result** — `submit_result` accepts a `game_id` parameter and returns `Error::GameIdMismatch` if it does not match the stored match `game_id`
- [x] **Duplicate game_id check removed** — removed the redundant second `game_id != game_id` check in `submit_result` that appeared after the state check
- [x] **game_id TTL extension fixed** — `create_match` now uses `m.game_id.clone()` consistently for both `set` and `extend_ttl` calls on `DataKey::GameId`, avoiding a use-after-move
- [x] **Either player can cancel** — `cancel_match` allows both `player1` and `player2` to cancel a pending match
- [x] **Admin role** — `initialize` accepts an `admin: Address`; `pause()` and `unpause()` are admin-only
- [x] **Oracle rotation** — `update_oracle(new_oracle)` is admin-only
- [x] **TTL extension** — all persistent writes call `extend_ttl` with `MATCH_TTL_LEDGERS = 518_400`
- [x] **Overflow guard** — `MatchCount` incremented with `checked_add(1)`
- [x] **Explicit escrow balance logic** — `get_escrow_balance` uses explicit match instead of bool-to-integer casting
- [x] **On-chain events** — `create_match`, `deposit`, `submit_result`, and `cancel_match` all emit events

## Error Variants

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `MatchNotFound` | Match ID does not exist |
| 2 | `AlreadyFunded` | Player already deposited |
| 3 | `NotFunded` | Both players have not deposited |
| 4 | `Unauthorized` | Caller lacks required authorization |
| 5 | `InvalidState` | Operation not allowed in current state |
| 6 | `AlreadyExists` | Match ID collision |
| 7 | `AlreadyInitialized` | Contract already initialized |
| 8 | `Overflow` | Match counter overflow |
| 9 | `ContractPaused` | Contract is paused |
| 10 | `InvalidAmount` | Stake amount ≤ 0 |
| 11 | `InvalidGameId` | Game ID exceeds 64 bytes |
| 12 | `InvalidPlayers` | player1 == player2 |
| 13 | `GameIdMismatch` | Oracle submitted result for wrong game_id |
| 14 | `DuplicateGameId` | game_id already used in another match |
