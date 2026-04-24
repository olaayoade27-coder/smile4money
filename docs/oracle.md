# Oracle Design

The oracle is the bridge between chess platform APIs (Lichess, Chess.com) and the Soroban smart contracts. It is the only component that can submit match results to the escrow contract.

## Role

The oracle solves the fundamental problem of bringing off-chain game results on-chain in a trustless way. Without it, players would need to manually report results — creating an obvious incentive to lie.

The oracle:
1. Monitors chess platform APIs for game completion
2. Verifies the result is final and unambiguous
3. Submits the result to the Oracle Contract on-chain
4. Calls `submit_result` on the Escrow Contract to trigger payout

## Architecture

```
Lichess API ──────┐
                  ├──► Off-chain Oracle Service ──► Oracle Contract
Chess.com API ────┘           │
                              └──────────────────► Escrow Contract
                                                   (submit_result)
```

The off-chain oracle service is a trusted backend process. Its Stellar address is registered in both contracts at initialization time. Only this address can submit results.

## Oracle Contract

The Oracle Contract (`contracts/oracle`) is an on-chain result registry. It provides a verifiable, immutable record of match outcomes.

### Why a separate oracle contract?

Storing results in a dedicated contract provides:
- An auditable history of all submitted results, independent of the escrow contract
- The ability to query results without going through the escrow contract
- A clean separation of concerns — the oracle contract handles result storage, the escrow contract handles funds

### Functions

**`initialize(admin: Address)`**

Sets the admin address (the off-chain oracle service). Can only be called once — a second call panics with `"Contract already initialized"`.

**`submit_result(match_id: u64, game_id: String, result: MatchResult)`**

Submits a verified result for a match. Requires admin auth. Rejects duplicate submissions with `Error::AlreadySubmitted`. Emits a `("oracle", "result")` event.

**`get_result(match_id: u64) -> ResultEntry`**

Returns the stored result. Returns `Error::ResultNotFound` if no result has been submitted.

**`has_result(match_id: u64) -> bool`**

Returns true if a result exists for the given match ID.

### Result types

```rust
enum MatchResult {
    Player1Wins,
    Player2Wins,
    Draw,
}

struct ResultEntry {
    game_id: String,   // the chess platform game ID
    result: MatchResult,
}
```

## Result Submission Flow

1. Off-chain oracle service detects game completion via Lichess/Chess.com API
2. Oracle service calls `OracleContract::submit_result(match_id, game_id, result)` — stores result on-chain
3. Oracle service calls `EscrowContract::submit_result(match_id, winner, caller)` — triggers payout
4. Escrow contract verifies `caller == oracle` address, then executes payout

```
Oracle Service
    │
    ├─1─► OracleContract::submit_result(match_id, game_id, result)
    │         └── stores ResultEntry, emits ("oracle", "result") event
    │
    └─2─► EscrowContract::submit_result(match_id, winner, caller)
              └── verifies caller == oracle
              └── executes payout
              └── emits ("match", "completed") event
```

## Platform Integration

### Lichess

Lichess provides a free, open API with no rate limiting for game lookups.

- Game endpoint: `https://lichess.org/api/game/{game_id}`
- Result field: `winner` (`white` | `black` | absent for draw)
- Status field: must be a terminal status (`mate`, `resign`, `stalemate`, `draw`, `timeout`, `outoftime`, `cheat`, `noStart`, `unknownFinish`, `variantEnd`)

The oracle service maps Lichess colors to match players using the `game_id` stored in the `Match` record.

### Chess.com

Chess.com provides a developer API for game lookups.

- Game endpoint: `https://api.chess.com/pub/game/{game_id}`
- Requires `CHESSDOTCOM_API_KEY` in environment
- Result field: `pgn` headers or `accuracies` — parse `[Result "1-0"]`, `[Result "0-1"]`, or `[Result "1/2-1/2"]`

## Security Properties

- **Single trusted oracle**: Only the registered oracle address can submit results. Any other caller receives `Error::Unauthorized`.
- **No double submission**: The oracle contract rejects duplicate results for the same `match_id` with `Error::AlreadySubmitted`.
- **Immutable results**: Once submitted, a result cannot be changed or deleted.
- **TTL-protected storage**: Result entries have a ~30-day TTL extended on write, preventing expiry during active matches.

## Configuration

Set the following environment variables for the oracle service:

```env
LICHESS_API_TOKEN=<your-lichess-api-token>
CHESSDOTCOM_API_KEY=<your-chessdotcom-api-key>
CONTRACT_ORACLE=<oracle-contract-id>
CONTRACT_ESCROW=<escrow-contract-id>
```

## Limitations and Future Work

- The current design requires the oracle service to call both contracts separately. A future improvement would have the oracle contract call the escrow contract directly via cross-contract invocation, removing the need for two separate transactions.
- The oracle is currently a single point of trust. A multi-sig oracle or decentralized oracle network (e.g., using threshold signatures) would further reduce trust assumptions.
- There is no on-chain timeout mechanism. If the oracle service goes offline, active matches remain stuck. A future version should allow players to claim a refund after a configurable timeout.
