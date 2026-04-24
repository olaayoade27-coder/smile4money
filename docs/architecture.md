# Architecture Overview

smile4money is a trustless chess wagering platform built on Stellar Soroban smart contracts. This document describes the system components, data flow, and design decisions.

## Components

```
┌─────────────────────────────────────────────────────────────┐
│                        Players                              │
│              (Stellar wallet holders)                       │
└────────────────────┬────────────────────────────────────────┘
                     │ create_match / deposit / cancel_match
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  Escrow Contract                            │
│                                                             │
│  - Holds stakes in XLM or USDC                             │
│  - Manages match lifecycle (Pending → Active → Completed)  │
│  - Executes payouts on verified result                      │
│  - Admin pause/unpause controls                             │
└────────────────────┬────────────────────────────────────────┘
                     │ submit_result(match_id, winner, caller)
                     ▲
┌─────────────────────────────────────────────────────────────┐
│                  Oracle Contract                            │
│                                                             │
│  - Stores verified match results on-chain                  │
│  - Admin-controlled (off-chain oracle service)             │
│  - Emits events on result submission                        │
└────────────────────┬────────────────────────────────────────┘
                     │ polls game result
                     ▲
┌─────────────────────────────────────────────────────────────┐
│              Off-chain Oracle Service                       │
│                                                             │
│  - Watches Lichess / Chess.com APIs                        │
│  - Verifies game completion                                 │
│  - Calls oracle contract and escrow contract               │
└─────────────────────────────────────────────────────────────┘
```

## Smart Contracts

### Escrow Contract (`contracts/escrow`)

The escrow contract is the core of the platform. It manages the full match lifecycle.

**State machine:**

```
Pending ──(both deposit)──► Active ──(submit_result)──► Completed
   │
   └──(cancel_match)──► Cancelled
```

**Key functions:**

| Function | Description |
|---|---|
| `initialize(oracle, admin)` | One-time setup. Sets trusted oracle and admin addresses. |
| `create_match(player1, player2, stake_amount, token, game_id, platform)` | Creates a new match in `Pending` state. |
| `deposit(match_id, player)` | Player deposits their stake. Match becomes `Active` when both have deposited. |
| `submit_result(match_id, winner, caller)` | Oracle submits result and triggers payout. |
| `cancel_match(match_id, caller)` | Either player cancels a `Pending` match. Refunds any deposits. |
| `get_match(match_id)` | Read a match by ID. |
| `is_funded(match_id)` | Returns true when both players have deposited. |
| `get_escrow_balance(match_id)` | Returns total escrowed amount (0, 1×, or 2× stake). |
| `pause()` / `unpause()` | Admin-only emergency controls. |

**Storage:**

All `Match` records are stored in persistent storage with a TTL of ~30 days (`MATCH_TTL_LEDGERS = 518_400` ledgers at 5s/ledger). TTL is extended on every state-changing write.

**Data keys:**

```rust
enum DataKey {
    Match(u64),   // persistent — match record
    MatchCount,   // instance — monotonic counter
    Oracle,       // instance — trusted oracle address
    Admin,        // instance — admin address
    Paused,       // instance — pause flag
}
```

### Oracle Contract (`contracts/oracle`)

The oracle contract is a simple result registry. The off-chain oracle service writes verified results here, and the escrow contract reads from it.

**Key functions:**

| Function | Description |
|---|---|
| `initialize(admin)` | One-time setup. Sets the admin (oracle service) address. |
| `submit_result(match_id, game_id, result)` | Admin submits a verified result. Idempotent guard prevents double-submission. |
| `get_result(match_id)` | Returns the stored `ResultEntry` for a match. |
| `has_result(match_id)` | Returns true if a result has been submitted. |

**Storage:**

Results are stored in persistent storage with the same 30-day TTL as match records.

## Data Model

### Match

```rust
struct Match {
    id: u64,
    player1: Address,
    player2: Address,
    stake_amount: i128,
    token: Address,          // XLM or USDC token contract
    game_id: String,         // Lichess/Chess.com game ID (max 64 chars)
    platform: Platform,      // Lichess | ChessDotCom
    state: MatchState,       // Pending | Active | Completed | Cancelled
    player1_deposited: bool,
    player2_deposited: bool,
    created_ledger: u32,     // ledger sequence at creation
}
```

### ResultEntry (Oracle)

```rust
struct ResultEntry {
    game_id: String,
    result: MatchResult,     // Player1Wins | Player2Wins | Draw
}
```

## Payout Logic

- **Player1 wins**: full pot (`2 × stake_amount`) transferred to `player1`
- **Player2 wins**: full pot transferred to `player2`
- **Draw**: each player receives their original `stake_amount` back

## Events

All state changes emit on-chain events for off-chain indexers and frontends.

| Contract | Topic | Data |
|---|---|---|
| Escrow | `("match", "created")` | `(match_id, player1, player2, stake_amount)` |
| Escrow | `("match", "activated")` | `match_id` |
| Escrow | `("match", "completed")` | `(match_id, winner)` |
| Escrow | `("match", "cancelled")` | `match_id` |
| Escrow | `("admin", "paused")` | `()` |
| Escrow | `("admin", "unpaused")` | `()` |
| Oracle | `("oracle", "result")` | `(match_id, result)` |

## Network Configuration

Supported networks are defined in `environments.toml`:

| Network | RPC |
|---|---|
| `testnet` | `https://soroban-testnet.stellar.org` |
| `mainnet` | Stellar mainnet |
| `futurenet` | Stellar futurenet |
| `standalone` | Local development |
