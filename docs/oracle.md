# Oracle Design

## Overview

The oracle is the bridge between off-chain chess platforms (Lichess, Chess.com) and the on-chain escrow contract. It is the only address authorised to call `submit_result` on the escrow contract.

## Components

### Oracle Contract (`contracts/oracle`)

An on-chain Soroban contract that:

- Stores verified results keyed by `match_id`
- Accepts result submissions only from the registered admin (the oracle service key)
- Prevents duplicate submissions for the same `match_id`
- Emits an on-chain event for every accepted result

### Off-chain Oracle Service

A backend process that:

1. Monitors the escrow contract for `("match", "activated")` events
2. Extracts `game_id` and `platform` from the match record
3. Polls the appropriate chess platform API until the game is finished
4. Submits the result to the Oracle Contract using the admin key
5. Calls `submit_result` on the Escrow Contract to trigger payout

## Result Submission Flow

```
Chess platform API
       │  game finished
       ▼
Oracle Service
  1. fetch game result
  2. map to MatchResult enum
  3. oracle_contract.submit_result(match_id, game_id, result)
  4. escrow_contract.submit_result(match_id, winner, oracle_address)
       │
       ▼
Escrow Contract
  - verifies caller == stored oracle address
  - verifies match state == Active
  - executes token payout
  - sets state = Completed
  - emits ("match", "completed") event
```

## Result Types

| Oracle `MatchResult` | Escrow `Winner` | Payout |
|----------------------|-----------------|--------|
| `Player1Wins` | `Player1` | Full pot to player1 |
| `Player2Wins` | `Player2` | Full pot to player2 |
| `Draw` | `Draw` | `stake_amount` returned to each player |

## Supported Platforms

| Platform | Enum Variant | API |
|----------|-------------|-----|
| Lichess | `Platform::Lichess` | `https://lichess.org/api/game/{id}` |
| Chess.com | `Platform::ChessDotCom` | `https://api.chess.com/pub/game/{id}` |

## Oracle Contract API

```
initialize(admin: Address)
submit_result(match_id: u64, game_id: String, result: MatchResult) -> Result<(), Error>
get_result(match_id: u64) -> Result<ResultEntry, Error>
has_result(match_id: u64) -> bool
```

### Errors

| Error | Code | Meaning |
|-------|------|---------|
| `Unauthorized` | 1 | Caller is not the admin |
| `AlreadySubmitted` | 2 | Result already exists for this match |
| `ResultNotFound` | 3 | No result stored for this match |
| `AlreadyInitialized` | 4 | Contract has already been initialized |

## Security Properties

- The oracle admin key is the only address that can submit results; any other caller is rejected with `Error::Unauthorized`.
- Once a result is submitted it is immutable — `AlreadySubmitted` prevents overwriting.
- The escrow contract independently verifies the caller against its stored oracle address before executing any payout.
- The oracle contract and escrow contract are separate deployments; a compromised oracle contract does not grant direct access to escrow funds.

## Configuration

Set the oracle admin key in `.env`:

```env
ORACLE_ADMIN_SECRET=<stellar-secret-key>
```

The oracle address is registered in the escrow contract at deploy time:

```bash
stellar contract invoke --id $CONTRACT_ESCROW \
  -- initialize \
  --oracle $ORACLE_ADDRESS \
  --admin $ADMIN_ADDRESS
```
