# API Reference

Complete reference for all smart contract functions, types, and errors.

## Escrow Contract

### Initialization

#### `initialize`

Initialize the escrow contract with oracle and admin addresses.

**Signature:**
```rust
pub fn initialize(env: Env, oracle: Address, admin: Address)
```

**Parameters:**
- `oracle`: Address of the trusted oracle contract
- `admin`: Address of the contract administrator

**Behavior:**
- Sets the oracle address for result verification
- Sets the admin address for emergency controls
- Initializes match counter to 0
- Sets paused state to false
- Panics if already initialized

**Authorization:** None required (only callable once)

**Errors:**
- Panics with "Contract already initialized" if called twice

---

### Admin Functions

#### `pause`

Pause the contract to prevent new matches, deposits, and result submissions.

**Signature:**
```rust
pub fn pause(env: Env) -> Result<(), Error>
```

**Behavior:**
- Sets paused flag to true
- Blocks create_match, deposit, and submit_result
- Emits "admin.paused" event

**Authorization:** Requires admin signature

**Errors:**
- `Error::Unauthorized`: Caller is not the admin

---

#### `unpause`

Resume normal contract operations.

**Signature:**
```rust
pub fn unpause(env: Env) -> Result<(), Error>
```

**Behavior:**
- Sets paused flag to false
- Re-enables all contract functions
- Emits "admin.unpaused" event

**Authorization:** Requires admin signature

**Errors:**
- `Error::Unauthorized`: Caller is not the admin

---

#### `update_oracle`

Rotate the trusted oracle address.

**Signature:**
```rust
pub fn update_oracle(env: Env, new_oracle: Address) -> Result<(), Error>
```

**Parameters:**
- `new_oracle`: Address of the replacement oracle

**Behavior:**
- Replaces the stored oracle address with `new_oracle`
- Emits "admin.oracle" event with the new oracle address

**Authorization:** Requires admin signature

**Errors:**
- `Error::Unauthorized`: Caller is not the admin

**Example:**
```rust
escrow.update_oracle(&new_oracle_addr);
```

---

### Match Management

#### `create_match`

Create a new betting match.

**Signature:**
```rust
pub fn create_match(
    env: Env,
    player1: Address,
    player2: Address,
    stake_amount: i128,
    token: Address,
    game_id: String,
    platform: Platform,
) -> Result<u64, Error>
```

**Parameters:**
- `player1`: Address of the match creator (must sign transaction)
- `player2`: Address of the opponent
- `stake_amount`: Amount each player must deposit (in token's smallest unit)
- `token`: Address of the token contract (e.g., XLM, USDC)
- `game_id`: Unique identifier from chess platform (max 64 bytes)
- `platform`: Chess platform enum (Lichess or ChessDotCom)

**Returns:**
- `u64`: Unique match ID

**Behavior:**
- Validates stake_amount > 0
- Validates game_id length ≤ 64 bytes
- Creates match in Pending state
- Increments match counter with overflow check
- Extends TTL to 30 days
- Emits "match.created" event with (match_id, player1, player2, stake_amount)

**Authorization:** Requires player1 signature

**Errors:**
- `Error::ContractPaused`: Contract is paused
- `Error::InvalidAmount`: stake_amount ≤ 0
- `Error::InvalidPlayers`: player1 and player2 are the same address
- `Error::InvalidGameId`: game_id exceeds 64 bytes
- `Error::DuplicateGameId`: game_id is already used in another match
- `Error::AlreadyExists`: Match ID collision (extremely rare)
- `Error::Overflow`: Match counter overflow (practically impossible)

**Example:**
```rust
let match_id = escrow.create_match(
    &player1_addr,
    &player2_addr,
    &1_000_0000, // 100 XLM (7 decimals)
    &xlm_token_addr,
    &String::from_str(&env, "lichess_abc123"),
    &Platform::Lichess,
);
```

---

#### `deposit`

Deposit stake into escrow for a match.

**Signature:**
```rust
pub fn deposit(env: Env, match_id: u64, player: Address) -> Result<(), Error>
```

**Parameters:**
- `match_id`: ID of the match to deposit into
- `player`: Address making the deposit (must be player1 or player2)

**Behavior:**
- Validates match exists and is in Pending state
- Validates caller is player1 or player2
- Transfers stake_amount tokens from player to contract
- Marks player as deposited
- If both players deposited, transitions to Active state
- Extends TTL to 30 days
- Emits "match.activated" event if both deposited

**Authorization:** Requires player signature

**Errors:**
- `Error::ContractPaused`: Contract is paused
- `Error::MatchNotFound`: Invalid match_id
- `Error::InvalidState`: Match is not Pending
- `Error::Unauthorized`: Caller is not player1 or player2
- `Error::AlreadyFunded`: Player already deposited

**Example:**
```rust
// Player 1 deposits
escrow.deposit(&match_id, &player1_addr);

// Player 2 deposits (match becomes Active)
escrow.deposit(&match_id, &player2_addr);
```

---

#### `cancel_match`

Cancel a pending match and refund any deposits.

**Signature:**
```rust
pub fn cancel_match(env: Env, match_id: u64, caller: Address) -> Result<(), Error>
```

**Parameters:**
- `match_id`: ID of the match to cancel
- `caller`: Address requesting cancellation (must be player1 or player2)

**Behavior:**
- Validates match is in Pending state
- Validates caller is player1 or player2
- Refunds player1 if they deposited
- Refunds player2 if they deposited
- Transitions to Cancelled state
- Extends TTL to 30 days
- Emits "match.cancelled" event

**Authorization:** Requires caller signature (player1 or player2)

**Errors:**
- `Error::MatchNotFound`: Invalid match_id
- `Error::InvalidState`: Match is not Pending (already Active, Completed, or Cancelled)
- `Error::Unauthorized`: Caller is not player1 or player2

**Example:**
```rust
// Either player can cancel
escrow.cancel_match(&match_id, &player1_addr);
```

---

### Result Submission

#### `submit_result`

Submit verified match result and execute payout.

**Signature:**
```rust
pub fn submit_result(
    env: Env,
    match_id: u64,
    game_id: String,
    winner: Winner,
    caller: Address,
) -> Result<(), Error>
```

**Parameters:**
- `match_id`: ID of the match to finalize
- `game_id`: Chess platform game identifier — must match the `game_id` stored in the match
- `winner`: Result enum (Player1, Player2, or Draw)
- `caller`: Address submitting result (must be oracle)

**Behavior:**
- Validates caller is the trusted oracle
- Validates `game_id` matches the match's stored `game_id` (prevents cross-match result injection)
- Validates match is Active
- Validates both players deposited
- Executes payout based on winner:
  - Player1: Transfers 2x stake to player1
  - Player2: Transfers 2x stake to player2
  - Draw: Returns 1x stake to each player
- Transitions to Completed state
- Extends TTL to 30 days
- Emits "match.completed" event with (match_id, winner)

**Authorization:** Requires oracle signature

**Errors:**
- `Error::ContractPaused`: Contract is paused
- `Error::Unauthorized`: Caller is not the oracle
- `Error::MatchNotFound`: Invalid match_id
- `Error::GameIdMismatch`: Provided game_id does not match the match record
- `Error::InvalidState`: Match is not Active
- `Error::NotFunded`: Both players have not deposited

**Example:**
```rust
// Oracle submits Player1 win
escrow.submit_result(&match_id, &String::from_str(&env, "lichess_game123"), &Winner::Player1, &oracle_addr);
```

---

### Query Functions

#### `get_match`

Retrieve full match details.

**Signature:**
```rust
pub fn get_match(env: Env, match_id: u64) -> Result<Match, Error>
```

**Parameters:**
- `match_id`: ID of the match to query

**Returns:**
- `Match`: Complete match struct

**Errors:**
- `Error::MatchNotFound`: Invalid match_id

**Example:**
```rust
let match_data = escrow.get_match(&match_id);
assert_eq!(match_data.state, MatchState::Active);
```

---

#### `is_funded`

Check if both players have deposited.

**Signature:**
```rust
pub fn is_funded(env: Env, match_id: u64) -> Result<bool, Error>
```

**Parameters:**
- `match_id`: ID of the match to check

**Returns:**
- `bool`: true if both players deposited, false otherwise

**Errors:**
- `Error::MatchNotFound`: Invalid match_id

**Example:**
```rust
if escrow.is_funded(&match_id) {
    // Match is ready to start
}
```

---

#### `get_escrow_balance`

Get total tokens held in escrow for a match.

**Signature:**
```rust
pub fn get_escrow_balance(env: Env, match_id: u64) -> Result<i128, Error>
```

**Parameters:**
- `match_id`: ID of the match to check

**Returns:**
- `i128`: Total escrowed amount (0, 1x, or 2x stake_amount)

**Behavior:**
- Returns 0 if match is Completed or Cancelled
- Returns stake_amount if one player deposited
- Returns 2 * stake_amount if both players deposited

**Errors:**
- `Error::MatchNotFound`: Invalid match_id

**Example:**
```rust
let balance = escrow.get_escrow_balance(&match_id);
// balance = 0, stake_amount, or 2 * stake_amount
```

---

## Oracle Contract

### Initialization

#### `initialize`

Initialize the oracle contract with admin address.

**Signature:**
```rust
pub fn initialize(env: Env, admin: Address)
```

**Parameters:**
- `admin`: Address of the oracle service (only address that can submit results)

**Behavior:**
- Sets admin address
- Panics if already initialized

**Authorization:** None required (only callable once)

**Errors:**
- Panics with "Contract already initialized" if called twice

---

### Result Management

#### `submit_result`

Submit a verified match result.

**Signature:**
```rust
pub fn submit_result(
    env: Env,
    match_id: u64,
    game_id: String,
    result: MatchResult,
) -> Result<(), Error>
```

**Parameters:**
- `match_id`: ID of the match (from escrow contract)
- `game_id`: Chess platform game identifier
- `result`: Result enum (Player1Wins, Player2Wins, or Draw)

**Behavior:**
- Validates caller is admin
- Prevents duplicate submissions
- Stores result with TTL extension
- Emits "oracle.result" event with (match_id, result)

**Authorization:** Requires admin signature

**Errors:**
- `Error::Unauthorized`: Caller is not admin
- `Error::AlreadySubmitted`: Result already exists for this match_id

**Example:**
```rust
oracle.submit_result(
    &match_id,
    &String::from_str(&env, "lichess_abc123"),
    &MatchResult::Player1Wins,
);
```

---

#### `get_result`

Retrieve stored result for a match.

**Signature:**
```rust
pub fn get_result(env: Env, match_id: u64) -> Result<ResultEntry, Error>
```

**Parameters:**
- `match_id`: ID of the match to query

**Returns:**
- `ResultEntry`: Struct containing game_id and result

**Errors:**
- `Error::ResultNotFound`: No result submitted for this match_id

**Example:**
```rust
let result = oracle.get_result(&match_id);
assert_eq!(result.result, MatchResult::Player1Wins);
```

---

#### `has_result`

Check if a result exists for a match.

**Signature:**
```rust
pub fn has_result(env: Env, match_id: u64) -> bool
```

**Parameters:**
- `match_id`: ID of the match to check

**Returns:**
- `bool`: true if result exists, false otherwise

**Example:**
```rust
if oracle.has_result(&match_id) {
    let result = oracle.get_result(&match_id);
}
```

---

## Data Types

### Match

Complete match record.

```rust
pub struct Match {
    pub id: u64,
    pub player1: Address,
    pub player2: Address,
    pub stake_amount: i128,
    pub token: Address,
    pub game_id: String,
    pub platform: Platform,
    pub state: MatchState,
    pub player1_deposited: bool,
    pub player2_deposited: bool,
    pub created_ledger: u32,
}
```

**Fields:**
- `id`: Unique match identifier
- `player1`: Match creator address
- `player2`: Opponent address
- `stake_amount`: Amount each player deposits
- `token`: Token contract address
- `game_id`: Chess platform game identifier
- `platform`: Chess platform (Lichess or ChessDotCom)
- `state`: Current match state
- `player1_deposited`: Whether player1 deposited
- `player2_deposited`: Whether player2 deposited
- `created_ledger`: Ledger sequence at creation

---

### MatchState

Match lifecycle states.

```rust
pub enum MatchState {
    Pending,   // Created, awaiting deposits
    Active,    // Both deposited, game in progress
    Completed, // Result submitted, payout executed
    Cancelled, // Cancelled before activation
}
```

---

### Platform

Supported chess platforms.

```rust
pub enum Platform {
    Lichess,
    ChessDotCom,
}
```

---

### Winner

Match outcome for escrow contract.

```rust
pub enum Winner {
    Player1,
    Player2,
    Draw,
}
```

---

### MatchResult

Match outcome for oracle contract.

```rust
pub enum MatchResult {
    Player1Wins,
    Player2Wins,
    Draw,
}
```

---

### ResultEntry

Oracle result storage.

```rust
pub struct ResultEntry {
    pub game_id: String,
    pub result: MatchResult,
}
```

---

## Error Codes

### Escrow Contract Errors

```rust
pub enum Error {
    MatchNotFound = 1,      // Match ID does not exist
    AlreadyFunded = 2,      // Player already deposited
    NotFunded = 3,          // Both players have not deposited
    Unauthorized = 4,       // Caller lacks required authorization
    InvalidState = 5,       // Operation not allowed in current state
    AlreadyExists = 6,      // Match ID collision
    AlreadyInitialized = 7, // Contract already initialized
    Overflow = 8,           // Match counter overflow
    ContractPaused = 9,     // Contract is paused
    InvalidAmount = 10,     // Stake amount is invalid (≤ 0)
    InvalidGameId = 11,     // Game ID exceeds max length
    InvalidPlayers = 12,    // player1 == player2 in create_match
    GameIdMismatch = 13,    // Oracle submitted result for wrong game_id
    DuplicateGameId = 14,   // game_id already used in another match
}
```

### Oracle Contract Errors

```rust
pub enum Error {
    Unauthorized = 1,       // Caller is not admin
    AlreadySubmitted = 2,   // Result already exists
    ResultNotFound = 3,     // No result for match_id
    AlreadyInitialized = 4, // Contract already initialized
}
```

---

## Events

### Escrow Contract Events

#### match.created
Emitted when a new match is created.

**Topics:** `("match", "created")`

**Data:** `(match_id: u64, player1: Address, player2: Address, stake_amount: i128)`

---

#### match.activated
Emitted when both players deposit and match becomes Active.

**Topics:** `("match", "activated")`

**Data:** `match_id: u64`

---

#### match.deposit
Emitted on every individual player deposit.

**Topics:** `("match", "deposit")`

**Data:** `(match_id: u64, player: Address)`

---

#### match.completed
Emitted when result is submitted and payout executed.

**Topics:** `("match", "completed")`

**Data:** `(match_id: u64, winner: Winner)`

---

#### match.cancelled
Emitted when a match is cancelled.

**Topics:** `("match", "cancelled")`

**Data:** `match_id: u64`

---

#### admin.paused
Emitted when contract is paused.

**Topics:** `("admin", "paused")`

**Data:** `()`

---

#### admin.unpaused
Emitted when contract is unpaused.

**Topics:** `("admin", "unpaused")`

**Data:** `()`

---

#### admin.oracle
Emitted when the oracle address is rotated via `update_oracle`.

**Topics:** `("admin", "oracle")`

**Data:** `new_oracle: Address`

---

### Oracle Contract Events

#### oracle.result
Emitted when a result is submitted.

**Topics:** `("oracle", "result")`

**Data:** `(match_id: u64, result: MatchResult)`

---

## Constants

### Escrow Contract

```rust
const MATCH_TTL_LEDGERS: u32 = 518_400;  // ~30 days at 5s/ledger
const MAX_GAME_ID_LEN: u32 = 64;         // Maximum game_id byte length
```

### Oracle Contract

```rust
const MATCH_TTL_LEDGERS: u32 = 518_400;  // ~30 days at 5s/ledger
```

---

## Usage Examples

### Complete Match Flow

```rust
// 1. Initialize contracts
escrow.initialize(&oracle_addr, &admin_addr);
oracle.initialize(&oracle_service_addr);

// 2. Create match
let match_id = escrow.create_match(
    &player1,
    &player2,
    &100_0000000, // 100 XLM
    &xlm_token,
    &String::from_str(&env, "lichess_game123"),
    &Platform::Lichess,
);

// 3. Players deposit
escrow.deposit(&match_id, &player1);
escrow.deposit(&match_id, &player2);

// 4. Check match is funded
assert!(escrow.is_funded(&match_id));

// 5. Players play chess game...

// 6. Oracle submits result
oracle.submit_result(
    &match_id,
    &String::from_str(&env, "lichess_game123"),
    &MatchResult::Player1Wins,
);

// 7. Oracle triggers payout
escrow.submit_result(&match_id, &String::from_str(&env, "lichess_game123"), &Winner::Player1, &oracle_addr);

// 8. Verify completion
let match_data = escrow.get_match(&match_id);
assert_eq!(match_data.state, MatchState::Completed);
```

### Cancellation Flow

```rust
// Create match
let match_id = escrow.create_match(...);

// Player1 deposits
escrow.deposit(&match_id, &player1);

// Player2 decides not to play
escrow.cancel_match(&match_id, &player2);

// Player1 gets refund automatically
```

### Emergency Pause

```rust
// Admin pauses contract
escrow.pause();

// All operations blocked
assert!(escrow.try_create_match(...).is_err());

// Admin unpauses
escrow.unpause();

// Operations resume
```
