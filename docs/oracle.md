# Oracle Design

The oracle is the bridge between off-chain chess game results and the on-chain escrow contract. This document describes how it works, its trust model, and its failure modes.

## Overview

smile4money uses a two-layer oracle design:

1. **Oracle Contract** (`contracts/oracle`) — An on-chain result registry. Stores verified results keyed by `match_id`. Admin-gated so only the trusted oracle service can write.
2. **Off-chain Oracle Service** — A backend process that polls Lichess and Chess.com APIs, verifies game outcomes, and submits results to both the Oracle Contract and the Escrow Contract.

## Data Flow

```
Chess.com / Lichess API
         │
         │  GET /game/{game_id}
         ▼
  Off-chain Oracle Service
         │
         ├── submit_result(match_id, game_id, result)  ──▶  Oracle Contract
         │                                                   (stores result on-chain)
         │
         └── submit_result(match_id, winner, caller)   ──▶  Escrow Contract
                                                            (executes payout)
```

## Oracle Contract API

### `initialize(admin: Address)`

Sets the oracle service address. Can only be called once — subsequent calls panic.

### `submit_result(match_id: u64, game_id: String, result: MatchResult) -> Result<(), Error>`

Stores a verified result on-chain. Requires admin auth. Rejects duplicate submissions (`Error::AlreadySubmitted`). Emits an `oracle / result` event.

`MatchResult` variants:
- `Player1Wins`
- `Player2Wins`
- `Draw`

### `get_result(match_id: u64) -> Result<ResultEntry, Error>`

Returns the stored `ResultEntry` for a match, or `Error::ResultNotFound`.

### `has_result(match_id: u64) -> bool`

Returns `true` if a result has been submitted for the given match ID.

## Off-chain Oracle Service

The oracle service is responsible for:

1. **Watching** — Polling Lichess (`/api/game/{id}`) and Chess.com (`/pub/player/{user}/games/{yyyy}/{mm}`) for game completion.
2. **Verifying** — Confirming the game is finished and mapping the result to the correct `match_id` stored in the escrow contract.
3. **Submitting** — Calling `submit_result` on the Oracle Contract (for auditability) and `submit_result` on the Escrow Contract (to trigger payout).

### Environment Variables

```env
LICHESS_API_TOKEN=<your-lichess-api-token>
CHESSDOTCOM_API_KEY=<your-chessdotcom-api-key>
CONTRACT_ORACLE=<oracle-contract-id>
CONTRACT_ESCROW=<escrow-contract-id>
STELLAR_RPC_URL=https://soroban-testnet.stellar.org
```

## Trust Model

The oracle is the only address authorized to call `submit_result` on the Escrow Contract. This means:

- If the oracle service is compromised, it can submit incorrect results and redirect payouts.
- The oracle address can be rotated via `update_oracle(new_oracle)` on the Escrow Contract (admin-only).
- All oracle submissions are recorded on-chain and emit events, making any manipulation auditable.

## Failure Modes

| Scenario | Behaviour |
|---|---|
| Oracle service goes offline | Match stays `Active` indefinitely. Players can wait or the admin can intervene. |
| Oracle submits wrong `match_id` | Escrow rejects if match is not `Active`. Oracle contract records the result regardless. |
| Duplicate result submission | Oracle contract returns `Error::AlreadySubmitted` and rejects. |
| Oracle key compromised | Admin rotates oracle address via `update_oracle`. Existing completed matches are unaffected. |

## Platforms Supported

| Platform | API | `Platform` enum value |
|---|---|---|
| Lichess | `https://lichess.org/api/game/{id}` | `Platform::Lichess` |
| Chess.com | `https://api.chess.com/pub/...` | `Platform::ChessDotCom` |
