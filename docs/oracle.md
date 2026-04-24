# Oracle Design

## Overview

The oracle is the bridge between off-chain chess platforms (Lichess, Chess.com) and the on-chain escrow contract. It is the only address authorised to call `submit_result` on the escrow contract.

## Components

### Off-chain Oracle Service

A backend process that:

1. Monitors active matches by watching the `match/activated` event emitted by the escrow contract.
2. Polls the Lichess or Chess.com API for the linked `game_id` until the game reaches a terminal state (win, loss, draw, abort).
3. Maps the platform result to the `Winner` enum (`Player1`, `Player2`, `Draw`).
4. Calls `escrow.submit_result(match_id, game_id, winner, oracle_address)` on-chain.
5. Optionally records the result in the Oracle Contract for auditability.

### Oracle Contract (`contracts/oracle`)

An immutable on-chain log. The oracle service's admin key calls `submit_result` to store a `ResultEntry { game_id, result }` keyed by `match_id`. This provides:

- A tamper-evident audit trail independent of the escrow contract.
- A queryable record for frontends and indexers via `get_result` / `has_result`.

## Result Flow

```
Chess platform API
       │
       │  game reaches terminal state
       ▼
Oracle Service
       │
       ├─► OracleContract.submit_result(match_id, game_id, result)
       │         stores ResultEntry, emits oracle/result event
       │
       └─► EscrowContract.submit_result(match_id, game_id, winner, oracle)
                 verifies game_id matches stored match
                 executes payout
                 emits match/completed event
```

## Security Properties

- The escrow contract stores the oracle address at `initialize` time. Only the admin can rotate it via `update_oracle`.
- `submit_result` on the escrow contract requires `caller == oracle` and `caller.require_auth()`, so the oracle's Stellar key must sign the transaction.
- The `game_id` passed to `submit_result` is cross-checked against the `game_id` stored in the match record. A compromised oracle cannot redirect a result to a different match.
- The Oracle Contract deduplicates by `match_id` — a result can only be submitted once per match.

## Platform Mapping

| Platform | API | Result field |
|----------|-----|-------------|
| Lichess | `https://lichess.org/api/game/{id}` | `winner`: `"white"` / `"black"` / absent (draw) |
| Chess.com | `https://api.chess.com/pub/game/{id}` | `pgn` headers or `white.result` / `black.result` |

The oracle service maps white/black to `Player1`/`Player2` based on the username stored at match creation time (future work — see [Roadmap](roadmap.md)).

## Configuration

```env
LICHESS_API_TOKEN=<your-lichess-api-token>
CHESSDOTCOM_API_KEY=<your-chessdotcom-api-key>
CONTRACT_ESCROW=<escrow-contract-id>
CONTRACT_ORACLE=<oracle-contract-id>
```
