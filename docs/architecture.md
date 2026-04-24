# Architecture Overview

## System Design

Smile4Money is a decentralized chess betting platform built on Stellar's Soroban smart contracts. The system consists of two primary smart contracts and an off-chain oracle service that bridges chess platform APIs to on-chain settlement.

## Components

### 1. Escrow Contract

The escrow contract manages the entire lifecycle of a betting match:

- **Match Creation**: Players create matches by specifying stake amounts, token type, and linking to a chess game
- **Fund Management**: Holds player deposits in escrow until the match concludes
- **State Management**: Tracks match progression through Pending → Active → Completed/Cancelled states
- **Payout Execution**: Automatically distributes winnings based on oracle-verified results
- **Admin Controls**: Pause/unpause functionality for emergency situations

**Key Features:**
- Trustless escrow (no platform can withhold funds)
- Automatic payouts triggered by oracle
- Support for multiple token types (XLM, USDC, custom tokens)
- TTL management for persistent storage entries
- Event emission for off-chain indexing

### 2. Oracle Contract

The oracle contract serves as the trusted bridge between chess platforms and the blockchain:

- **Result Verification**: Stores verified match results from chess platform APIs
- **Admin-Only Submission**: Only the designated oracle service can submit results
- **Result Storage**: Maintains a permanent record of all match outcomes
- **Event Publishing**: Emits events when results are submitted

**Key Features:**
- Single admin model (oracle service address)
- Duplicate submission prevention
- TTL-extended persistent storage
- Event-driven architecture for real-time updates

### 3. Off-Chain Oracle Service (Not Yet Implemented)

The oracle service is a backend application that:

1. Monitors the escrow contract for new Active matches
2. Polls Lichess/Chess.com APIs for game results
3. Verifies game completion and outcome
4. Submits verified results to both oracle and escrow contracts
5. Triggers automatic payouts

**Planned Features:**
- Multi-platform support (Lichess, Chess.com)
- Result verification with multiple API calls
- Retry logic for failed submissions
- Rate limiting and API key management
- Monitoring and alerting

## Data Flow

### Match Creation Flow

```
1. Player1 calls create_match()
   ├─> Validates inputs (stake > 0, game_id length, etc.)
   ├─> Creates Match struct with Pending state
   ├─> Stores in persistent storage with TTL
   ├─> Emits "match.created" event
   └─> Returns match_id

2. Player1 calls deposit(match_id)
   ├─> Validates authorization
   ├─> Transfers tokens to contract
   ├─> Sets player1_deposited = true
   └─> Match remains Pending

3. Player2 calls deposit(match_id)
   ├─> Validates authorization
   ├─> Transfers tokens to contract
   ├─> Sets player2_deposited = true
   ├─> Updates state to Active
   ├─> Emits "match.activated" event
   └─> Players can now start their chess game
```

### Result Submission & Payout Flow

```
1. Players complete chess game on Lichess/Chess.com

2. Oracle service detects game completion
   ├─> Fetches game result from platform API
   ├─> Verifies game_id matches match record
   └─> Determines winner (Player1/Player2/Draw)

3. Oracle calls oracle.submit_result()
   ├─> Stores result in oracle contract
   ├─> Emits "oracle.result" event
   └─> Result is now verifiable on-chain

4. Oracle calls escrow.submit_result()
   ├─> Validates caller is trusted oracle
   ├─> Validates match is Active
   ├─> Calculates payout based on winner
   ├─> Transfers tokens from contract to winner(s)
   ├─> Updates match state to Completed
   ├─> Emits "match.completed" event
   └─> Payout complete
```

### Cancellation Flow

```
1. Either player calls cancel_match(match_id)
   ├─> Validates match is still Pending
   ├─> Validates caller is player1 or player2
   ├─> Refunds any deposited funds
   ├─> Updates state to Cancelled
   ├─> Emits "match.cancelled" event
   └─> Match is permanently cancelled
```

## Storage Architecture

### Escrow Contract Storage

**Instance Storage** (contract-level data):
- `Oracle`: Address of trusted oracle contract
- `Admin`: Address of contract administrator
- `MatchCount`: Monotonically increasing match ID counter
- `Paused`: Boolean flag for emergency pause

**Persistent Storage** (match-level data):
- `Match(u64)`: Full match record keyed by match_id
  - TTL: 30 days (518,400 ledgers at 5s/ledger)
  - Extended on every write operation

### Oracle Contract Storage

**Instance Storage**:
- `Admin`: Address of oracle service

**Persistent Storage**:
- `Result(u64)`: Result entry keyed by match_id
  - TTL: 30 days (518,400 ledgers)
  - Extended on write

## State Machine

### Match States

```
Pending
  ├─> Active (when both players deposit)
  ├─> Cancelled (when either player cancels before both deposit)
  └─> [Invalid transitions blocked]

Active
  ├─> Completed (when oracle submits result)
  └─> [Cannot cancel or re-deposit]

Completed
  └─> [Terminal state - no further transitions]

Cancelled
  └─> [Terminal state - no further transitions]
```

## Security Model

### Trust Assumptions

1. **Oracle Trust**: The system trusts the oracle service to:
   - Accurately fetch and verify game results
   - Submit results only for legitimate completed games
   - Not collude with players to submit false results

2. **Platform Trust**: The system trusts Lichess/Chess.com APIs to:
   - Provide accurate game results
   - Not be manipulated by players
   - Remain available and reliable

3. **Smart Contract Trust**: Players trust that:
   - The escrow contract will hold funds securely
   - Payouts will execute automatically when results are submitted
   - The admin cannot steal funds (only pause/unpause)

### Security Features

- **Authorization Checks**: All state-changing functions require proper authentication
- **State Validation**: Strict state machine prevents invalid transitions
- **Overflow Protection**: Checked arithmetic prevents integer overflow
- **Reentrancy Safety**: Soroban's execution model prevents reentrancy attacks
- **Admin Controls**: Emergency pause functionality for critical vulnerabilities
- **TTL Management**: Prevents data expiration during active matches

### Known Limitations

See [issues.md](../issues.md) for a comprehensive list of known bugs and security issues that need to be addressed before production deployment.

## Event Architecture

All major state changes emit events for off-chain indexing and real-time updates:

### Escrow Events

- `match.created`: New match created
- `match.activated`: Both players deposited, game can start
- `match.completed`: Result submitted, payout executed
- `match.cancelled`: Match cancelled, refunds issued
- `admin.paused`: Contract paused by admin
- `admin.unpaused`: Contract unpaused by admin

### Oracle Events

- `oracle.result`: New result submitted

Events enable:
- Real-time frontend updates
- Off-chain indexing for match history
- Analytics and monitoring
- User notifications

## Scalability Considerations

### Current Limitations

- **Sequential Match IDs**: Match IDs are sequential, creating a potential bottleneck
- **Single Oracle**: Only one oracle address can submit results
- **No Batch Operations**: Each match requires separate transactions
- **Storage Costs**: Each match consumes persistent storage with 30-day TTL

### Future Improvements

- **Multi-Oracle Support**: Allow multiple trusted oracles with consensus mechanism
- **Batch Result Submission**: Submit multiple results in one transaction
- **Match Archival**: Move completed matches to cheaper storage tier
- **Optimistic Rollups**: Batch multiple matches off-chain with fraud proofs

## Integration Points

### Frontend Integration

The frontend should:
1. Monitor events for real-time updates
2. Query match state before allowing deposits
3. Display escrow balance to users
4. Handle wallet connection and transaction signing
5. Show match history from indexed events

### Oracle Service Integration

The oracle service should:
1. Subscribe to `match.activated` events
2. Poll chess platform APIs for game completion
3. Verify game results with multiple API calls
4. Submit results to both oracle and escrow contracts
5. Handle retries and error cases
6. Monitor for failed transactions

### Wallet Integration

Users need:
- Stellar wallet (Freighter, Albedo, etc.)
- Sufficient token balance for stakes
- XLM for transaction fees
- Understanding of transaction signing

## Deployment Architecture

### Testnet Deployment

```
1. Deploy oracle contract
   └─> Initialize with oracle service address

2. Deploy escrow contract
   └─> Initialize with oracle address and admin address

3. Configure oracle service
   ├─> Set contract addresses
   ├─> Configure API keys
   └─> Start monitoring service

4. Deploy frontend
   ├─> Configure contract addresses
   └─> Connect to Stellar testnet RPC
```

### Mainnet Deployment

Additional considerations:
- Multi-signature admin control
- Gradual rollout with stake limits
- Comprehensive monitoring and alerting
- Bug bounty program
- Emergency response procedures
- Regular security audits

## Technology Stack

- **Smart Contracts**: Rust + Soroban SDK
- **Blockchain**: Stellar (Soroban)
- **Oracle Service**: (To be implemented - likely Node.js/Python)
- **Chess APIs**: Lichess API, Chess.com API
- **Frontend**: (To be implemented - likely React + Freighter wallet)
- **Testing**: Soroban SDK test framework
- **Build Tools**: Cargo, Soroban CLI, Stellar CLI
