# Architecture Overview

## System Components

```
┌─────────────────────────────────────────────────────────┐
│                      Players                            │
│           (Stellar wallets / frontend)                  │
└────────────────────┬────────────────────────────────────┘
                     │ create_match / deposit / cancel
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Escrow Contract                            │
│  - Holds stakes in persistent storage                   │
│  - Manages match lifecycle (Pending → Active →          │
│    Completed / Cancelled)                               │
│  - Executes payouts on verified result                  │
│  - Admin: pause / unpause / update_oracle               │
└────────────────────┬────────────────────────────────────┘
                     │ submit_result (oracle address only)
                     ▲
┌─────────────────────────────────────────────────────────┐
│              Oracle Service (off-chain)                 │
│  - Polls Lichess / Chess.com APIs                       │
│  - Verifies game result                                 │
│  - Calls escrow.submit_result(match_id, game_id, ...)   │
└────────────────────┬────────────────────────────────────┘
                     │ submit_result (admin address only)
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Oracle Contract (on-chain log)             │
│  - Stores ResultEntry { game_id, result } per match_id  │
│  - Emits oracle/result event                            │
│  - Provides get_result / has_result read API            │
└─────────────────────────────────────────────────────────┘
```

## Match Lifecycle

```
create_match()
     │
     ▼
  Pending ──── cancel_match() ──► Cancelled
     │
     │  player1 deposit + player2 deposit
     ▼
  Active
     │
     │  oracle submit_result()
     ▼
  Completed
```

## Contracts

### Escrow Contract (`contracts/escrow`)

Holds player stakes and executes payouts. Key storage:

| Key | Type | Description |
|-----|------|-------------|
| `Oracle` | `Address` | Trusted oracle address |
| `Admin` | `Address` | Admin address (pause / oracle rotation) |
| `MatchCount` | `u64` | Auto-incrementing match ID counter |
| `Paused` | `bool` | Circuit breaker flag |
| `Match(u64)` | `Match` | Match record keyed by ID |
| `GameId(String)` | `bool` | Used game IDs (dedup guard) |

### Oracle Contract (`contracts/oracle`)

Immutable on-chain log of verified match results. Key storage:

| Key | Type | Description |
|-----|------|-------------|
| `Admin` | `Address` | Oracle service address |
| `Result(u64)` | `ResultEntry` | Result keyed by match_id |

## Storage TTL

All persistent entries use `MATCH_TTL_LEDGERS = 518_400` (~30 days at 5 s/ledger) as both the TTL threshold and extend-to value. TTL is refreshed on every write to prevent expiry during active matches.

## Token Handling

The escrow contract is token-agnostic. Any SEP-41 / Stellar asset contract address can be used as the stake token. The token address is stored per match and used for all transfers.

## Events

| Contract | Topic | Data |
|----------|-------|------|
| Escrow | `match/created` | `(match_id, player1, player2, stake_amount)` |
| Escrow | `match/deposit` | `(match_id, player)` |
| Escrow | `match/activated` | `match_id` |
| Escrow | `match/completed` | `(match_id, winner)` |
| Escrow | `match/cancelled` | `match_id` |
| Escrow | `admin/paused` | `()` |
| Escrow | `admin/unpaused` | `()` |
| Escrow | `admin/oracle` | `new_oracle` |
| Oracle | `oracle/result` | `(match_id, result)` |
