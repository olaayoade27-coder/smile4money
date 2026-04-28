# Technical Deep Dive — smile4money

This document covers the internal mechanics of the two Soroban smart contracts that power smile4money: the **Escrow Contract** and the **Oracle Contract**.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Contract Storage Layout](#2-contract-storage-layout)
3. [Data Types](#3-data-types)
4. [Error Codes](#4-error-codes)
5. [Escrow Contract Internals](#5-escrow-contract-internals)
6. [Oracle Contract Internals](#6-oracle-contract-internals)
7. [Token Flow](#7-token-flow)
8. [Event Emissions](#8-event-emissions)
9. [Security Properties](#9-security-properties)
10. [Storage TTL Strategy](#10-storage-ttl-strategy)

---

## 1. System Overview

```
Players
  │  create_match / deposit / cancel_match
  ▼
Escrow Contract (Soroban)
  │  submit_result(match_id, game_id, winner, caller)
  ▲
Oracle Contract (Soroban)
  ▲
Off-chain Oracle Service
  │  polls Lichess / Chess.com APIs
```

The Escrow Contract holds player stakes and enforces the match lifecycle. The Oracle Contract is an on-chain record of verified game results. The off-chain Oracle Service bridges chess platform APIs to the on-chain contracts.

---

## 2. Contract Storage Layout

### Escrow Contract

All instance-storage keys are cheap to access and do not expire with match data.

| Key | Storage tier | Type | Description |
|-----|-------------|------|-------------|
| `DataKey::Oracle` | Instance | `Address` | Trusted oracle address; only this address may call `submit_result` |
| `DataKey::Admin` | Instance | `Address` | Admin address for pause/unpause and oracle rotation |
| `DataKey::MatchCount` | Instance | `u64` | Monotonically increasing match ID counter |
| `DataKey::Paused` | Instance | `bool` | Circuit-breaker flag; `true` blocks all state-changing operations |
| `DataKey::Match(u64)` | Persistent | `Match` | Full match record keyed by match ID |
| `DataKey::GameId(String)` | Persistent | `u64` | Maps a `game_id` string to the match ID that claimed it; prevents duplicate matches |

### Oracle Contract

| Key | Storage tier | Type | Description |
|-----|-------------|------|-------------|
| `DataKey::Admin` | Instance | `Address` | Oracle service address; only this address may call `submit_result` |
| `DataKey::Result(u64)` | Persistent | `ResultEntry` | Verified result keyed by match ID |

---

## 3. Data Types

### Escrow Contract

#### `Match`

```rust
pub struct Match {
    pub id: u64,
    pub player1: Address,
    pub player2: Address,
    pub stake_amount: i128,       // per-player stake in token's smallest unit
    pub token: Address,           // Stellar Asset Contract (SAC) address
    pub game_id: String,          // chess platform game identifier (max 64 bytes)
    pub platform: Platform,       // Lichess or ChessDotCom
    pub state: MatchState,
    pub player1_deposited: bool,
    pub player2_deposited: bool,
    pub created_ledger: u32,      // ledger sequence at creation
}
```

#### `MatchState`

```
Pending   → created, awaiting both deposits
Active    → both players deposited, game in progress
Completed → result submitted, payout executed
Cancelled → cancelled before activation
```

State transitions are strictly enforced. `deposit` is only accepted in `Pending`. `submit_result` is only accepted in `Active`. `cancel_match` is only accepted in `Pending`.

#### `Winner`

```rust
pub enum Winner { Player1, Player2, Draw }
```

#### `Platform`

```rust
pub enum Platform { Lichess, ChessDotCom }
```

### Oracle Contract

#### `ResultEntry`

```rust
pub struct ResultEntry {
    pub game_id: String,
    pub result: MatchResult,
}
```

#### `MatchResult`

```rust
pub enum MatchResult { Player1Wins, Player2Wins, Draw }
```

---

## 4. Error Codes

### Escrow Contract

| Variant | Code | Trigger |
|---------|------|---------|
| `MatchNotFound` | 1 | `match_id` does not exist in storage |
| `AlreadyFunded` | 2 | Player calls `deposit` a second time |
| `NotFunded` | 3 | `submit_result` called before both players deposited |
| `Unauthorized` | 4 | Caller is not the expected address (player, oracle, or admin) |
| `InvalidState` | 5 | Operation not valid for the match's current `MatchState` |
| `AlreadyExists` | 6 | Match ID collision (should not occur with checked counter) |
| `AlreadyInitialized` | 7 | `initialize` called on an already-initialized contract |
| `Overflow` | 8 | `MatchCount` would overflow `u64` |
| `ContractPaused` | 9 | State-changing call while contract is paused |
| `InvalidAmount` | 10 | `stake_amount <= 0` in `create_match` |
| `InvalidGameId` | 11 | `game_id` exceeds 64 bytes |
| `InvalidPlayers` | 12 | `player1 == player2` in `create_match` |
| `GameIdMismatch` | 13 | Oracle's `game_id` does not match the stored match `game_id` |
| `DuplicateGameId` | 14 | `game_id` already used in another match |

### Oracle Contract

| Variant | Code | Trigger |
|---------|------|---------|
| `Unauthorized` | 1 | Caller is not the admin |
| `AlreadySubmitted` | 2 | Result already stored for this `match_id` |
| `ResultNotFound` | 3 | No result stored for this `match_id` |
| `AlreadyInitialized` | 4 | `initialize` called on an already-initialized contract |
| `InvalidGameId` | 5 | `game_id` exceeds 64 bytes |

---

## 5. Escrow Contract Internals

### `initialize(oracle, admin)`

Writes `Oracle`, `Admin`, `MatchCount = 0`, and `Paused = false` to instance storage. Panics with `"Contract already initialized"` if `DataKey::Oracle` already exists. Uses a panic (not a structured error) so the transaction is unconditionally rejected with no side effects.

### `create_match(player1, player2, stake_amount, token, game_id, platform) → u64`

1. Requires `player1` auth.
2. Checks `ContractPaused`, `stake_amount > 0`, `player1 != player2`, `game_id.len() <= 64`.
3. Checks `DataKey::GameId(game_id)` does not exist in persistent storage.
4. Reads `MatchCount`, constructs a `Match` with `state = Pending`.
5. Writes `DataKey::Match(id)` and `DataKey::GameId(game_id)` to persistent storage with TTL.
6. Increments `MatchCount` using `checked_add(1)` to guard against overflow.
7. Emits `("match", "created")`.

### `deposit(match_id, player)`

1. Requires `player` auth.
2. Checks `ContractPaused`.
3. Loads match; checks `state == Pending`.
4. Verifies `player` is `player1` or `player2`; returns `Unauthorized` otherwise.
5. Checks the player has not already deposited (`AlreadyFunded`).
6. Calls `token::Client::transfer(player → contract, stake_amount)`.
7. Sets the player's `deposited` flag. If both flags are now `true`, transitions state to `Active` and emits `("match", "activated")`.
8. Emits `("match", "deposit")`.
9. Writes updated match with TTL refresh.

### `submit_result(match_id, game_id, winner, caller)`

1. Checks `ContractPaused`.
2. Loads `DataKey::Oracle`; compares `caller == oracle`; returns `Unauthorized` on mismatch.
3. Calls `caller.require_auth()`.
4. Loads match; verifies `m.game_id == game_id` (`GameIdMismatch` on mismatch).
5. Checks `state == Active` (`InvalidState` otherwise).
6. Checks both deposit flags are set (`NotFunded` otherwise).
7. Executes token transfer based on `winner`:
   - `Player1` → transfer `stake_amount * 2` to `player1`
   - `Player2` → transfer `stake_amount * 2` to `player2`
   - `Draw` → transfer `stake_amount` to each player
8. Sets `state = Completed`, writes match with TTL refresh.
9. Emits `("match", "completed")`.

### `cancel_match(match_id, caller)`

1. Loads match; checks `state == Pending` (`InvalidState` otherwise).
2. Verifies `caller` is `player1` or `player2` (`Unauthorized` otherwise).
3. Calls `caller.require_auth()`.
4. Refunds any deposits already made (player1 and/or player2 independently).
5. Sets `state = Cancelled`, writes match with TTL refresh.
6. Emits `("match", "cancelled")`.

### `get_escrow_balance(match_id) → i128`

Returns `0` for `Completed` or `Cancelled` matches. Otherwise uses explicit pattern matching on `(player1_deposited, player2_deposited)` to return `0`, `stake_amount`, or `2 * stake_amount`. Avoids fragile `bool as i128` casting.

### `update_oracle(new_oracle)` / `pause()` / `unpause()`

All three require `admin.require_auth()`. `update_oracle` overwrites `DataKey::Oracle` and emits `("admin", "oracle")`. `pause`/`unpause` set `DataKey::Paused` and emit `("admin", "paused"/"unpaused")`.

---

## 6. Oracle Contract Internals

### `initialize(admin)`

Returns `Error::AlreadyInitialized` (structured error, not panic) if `DataKey::Admin` already exists. Writes admin to instance storage. Emits `("oracle", "init")`.

### `submit_result(match_id, game_id, result)`

1. Loads admin; calls `admin.require_auth()`.
2. Validates `game_id.len() <= 64`.
3. Checks `DataKey::Result(match_id)` does not exist (`AlreadySubmitted` otherwise).
4. Writes `ResultEntry { game_id, result }` to persistent storage with TTL.
5. Emits `("oracle", "result")`.

The oracle contract is a **record store only** — it does not call back into the escrow contract. The off-chain oracle service calls both contracts independently. The escrow contract validates the oracle's identity via the stored `DataKey::Oracle` address.

### `get_result(match_id) → ResultEntry`

Read-only. Returns `Error::ResultNotFound` if no entry exists.

### `has_result(match_id) → bool`

Read-only. Returns `true` if a result entry exists for the given match ID.

---

## 7. Token Flow

All transfers use the Stellar Asset Contract (SAC) interface via `soroban_sdk::token::Client`. The `token` address is set per-match at creation time, supporting XLM and any SAC-compatible token.

```
create_match:   (no transfer — match record only)

deposit:        player  ──[stake_amount]──▶  escrow contract

submit_result:
  Player1 wins: escrow  ──[stake_amount × 2]──▶  player1
  Player2 wins: escrow  ──[stake_amount × 2]──▶  player2
  Draw:         escrow  ──[stake_amount]──▶  player1
                escrow  ──[stake_amount]──▶  player2

cancel_match:
  (if player1 deposited) escrow  ──[stake_amount]──▶  player1
  (if player2 deposited) escrow  ──[stake_amount]──▶  player2
```

The escrow contract never holds more than `2 × stake_amount` per match. Funds from different matches are co-mingled in the contract's token balance, but each match's accounting is tracked independently via the `player1_deposited` / `player2_deposited` flags and `stake_amount`.

---

## 8. Event Emissions

All events are emitted via `env.events().publish((topic1, topic2), data)`.

### Escrow Contract

| Topics | Data | Emitted by |
|--------|------|-----------|
| `("match", "created")` | `(match_id: u64, player1: Address, player2: Address, stake_amount: i128)` | `create_match` |
| `("match", "deposit")` | `(match_id: u64, player: Address)` | `deposit` |
| `("match", "activated")` | `match_id: u64` | `deposit` (when both players funded) |
| `("match", "completed")` | `(match_id: u64, winner: Winner)` | `submit_result` |
| `("match", "cancelled")` | `match_id: u64` | `cancel_match` |
| `("admin", "paused")` | `()` | `pause` |
| `("admin", "unpaused")` | `()` | `unpause` |
| `("admin", "oracle")` | `new_oracle: Address` | `update_oracle` |

### Oracle Contract

| Topics | Data | Emitted by |
|--------|------|-----------|
| `("oracle", "init")` | `admin: Address` | `initialize` |
| `("oracle", "result")` | `(match_id: u64, result: MatchResult)` | `submit_result` |

Off-chain indexers and frontends should subscribe to `("match", "activated")` to know when to start polling for a game result, and `("match", "completed")` to detect payouts without polling storage.

---

## 9. Security Properties

### Re-initialization Guard

Both contracts check for the existence of their primary key (`DataKey::Oracle` / `DataKey::Admin`) before writing. A second `initialize` call is rejected before any storage mutation occurs.

### Oracle Identity Verification

`submit_result` on the escrow contract compares the `caller` argument against the stored `DataKey::Oracle` address and calls `caller.require_auth()`. Both checks must pass. The `caller` parameter is explicit rather than using `env.invoker()` to make the authorization model clear and testable.

### Cross-Match Result Injection Prevention

`submit_result` requires a `game_id` parameter and compares it against `m.game_id` stored at match creation. A compromised oracle cannot redirect a payout by submitting a result for the correct `match_id` but a different game — the `GameIdMismatch` error fires before any token transfer.

### Duplicate Game ID Prevention

`create_match` writes `DataKey::GameId(game_id)` to persistent storage and checks for its existence before creating a match. The same chess game cannot be used to create two separate matches, preventing a double-payout attack.

### Overflow-Safe Match Counter

`MatchCount` is incremented with `checked_add(1).ok_or(Error::Overflow)?`. In Soroban release builds, integer overflow wraps silently; the checked arithmetic ensures the contract panics with a structured error instead of reusing match IDs.

### Pause Circuit Breaker

`create_match`, `deposit`, and `submit_result` all check `DataKey::Paused` at entry. `cancel_match` is intentionally **not** blocked by pause, so players can always recover their deposits even during an emergency pause.

### Admin / Oracle Key Separation

The admin key (pause, oracle rotation) and the oracle key (result submission) are separate addresses. Compromising the oracle key does not grant admin capabilities, and vice versa.

### No Admin Access to Funds

The admin address has no function that transfers tokens. Funds can only leave the escrow contract via `submit_result` (to the winner) or `cancel_match` (refund to depositors). The admin cannot drain the contract.

---

## 10. Storage TTL Strategy

Soroban persistent storage entries expire after their TTL elapses. All persistent writes in both contracts call `extend_ttl` immediately after the write:

```rust
env.storage().persistent().extend_ttl(
    &key,
    MATCH_TTL_LEDGERS,  // threshold: extend if TTL < this
    MATCH_TTL_LEDGERS,  // extend_to: new TTL value
);
```

`MATCH_TTL_LEDGERS = 518_400` corresponds to approximately 30 days at a 5-second ledger close time.

TTL is refreshed on every state-changing write to the same key:

- `DataKey::Match(id)` — refreshed on `create_match`, `deposit`, `submit_result`, `cancel_match`
- `DataKey::GameId(game_id)` — set once on `create_match`; not refreshed (game IDs are permanent)
- `DataKey::Result(match_id)` — set once on oracle `submit_result`; not refreshed

Instance storage (Oracle, Admin, MatchCount, Paused) does not use TTL extension because instance storage lifetime is tied to the contract instance itself.
