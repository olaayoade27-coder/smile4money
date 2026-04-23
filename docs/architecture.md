# Architecture Overview

smile4money is a trustless chess wagering platform built on Stellar Soroban smart contracts. This document describes the system components and how they interact.

## Components

```
┌─────────────────────────────────────────────────────────────┐
│                        Players                              │
│              (Stellar wallets / frontend)                   │
└────────────────────┬────────────────────────────────────────┘
                     │ create_match / deposit / cancel_match
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  Escrow Contract                            │
│                                                             │
│  • Holds XLM / USDC stakes in persistent storage           │
│  • Manages match lifecycle (Pending → Active → Completed)  │
│  • Executes payouts on verified result                      │
│  • Admin pause / unpause controls                          │
└────────────────────┬────────────────────────────────────────┘
                     │ submit_result (oracle address only)
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  Oracle Contract                            │
│                                                             │
│  • Stores verified match results on-chain                  │
│  • Admin-gated: only the oracle service can write          │
│  • Emits events for off-chain indexers                     │
└────────────────────┬────────────────────────────────────────┘
                     │ polls game APIs
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              Off-chain Oracle Service                       │
│                                                             │
│  • Watches Lichess / Chess.com APIs for game results       │
│  • Submits verified results to the Oracle Contract         │
│  • Calls submit_result on the Escrow Contract to pay out   │
└─────────────────────────────────────────────────────────────┘
```

## Match Lifecycle

```
create_match()
     │
     ▼
  Pending ──── deposit(player1) ──── deposit(player2) ──── Active
     │                                                        │
     └── cancel_match() ──── Cancelled          submit_result()
                                                              │
                                                         Completed
```

1. **Pending** — Match created, awaiting both deposits. Either player can cancel.
2. **Active** — Both players have deposited. Game is in progress. Cannot be cancelled.
3. **Completed** — Oracle submitted result, payout executed.
4. **Cancelled** — Cancelled before activation; any deposits refunded.

## Contracts

### Escrow Contract (`contracts/escrow`)

The core contract. Responsibilities:

- `initialize(oracle, admin)` — Sets the trusted oracle address and admin. One-time call.
- `create_match(player1, player2, stake_amount, token, game_id, platform)` — Creates a match record in persistent storage.
- `deposit(match_id, player)` — Transfers `stake_amount` tokens from the player to the contract. Transitions to `Active` when both players have deposited.
- `submit_result(match_id, winner, caller)` — Oracle-only. Executes payout to winner (or splits on draw) and marks match `Completed`.
- `cancel_match(match_id, caller)` — Either player can cancel a `Pending` match. Refunds any deposits.
- `pause()` / `unpause()` — Admin-only emergency controls.

### Oracle Contract (`contracts/oracle`)

A lightweight result registry. Responsibilities:

- `initialize(admin)` — Sets the oracle service address. One-time call.
- `submit_result(match_id, game_id, result)` — Admin-only. Stores a `ResultEntry` keyed by `match_id`.
- `get_result(match_id)` — Returns the stored `ResultEntry`.
- `has_result(match_id)` — Returns whether a result exists.

## Storage

Both contracts use Soroban **persistent storage** for match and result data, with TTL extended to `MATCH_TTL_LEDGERS` (~30 days at 5s/ledger) on every write. Instance storage holds contract-level config (oracle address, admin, match count, paused flag).

## Token Support

The escrow contract is token-agnostic — it accepts any SEP-41 compatible token address. v1.0 targets XLM; v1.1 adds USDC.

## Events

All state transitions emit on-chain events for off-chain indexers and frontends:

| Contract | Event topic          | Data                              |
|----------|----------------------|-----------------------------------|
| Escrow   | `match / created`    | `(match_id, player1, player2, stake_amount)` |
| Escrow   | `match / activated`  | `match_id`                        |
| Escrow   | `match / completed`  | `(match_id, winner)`              |
| Escrow   | `match / cancelled`  | `match_id`                        |
| Escrow   | `admin / paused`     | `()`                              |
| Escrow   | `admin / unpaused`   | `()`                              |
| Oracle   | `oracle / result`    | `(match_id, result)`              |
