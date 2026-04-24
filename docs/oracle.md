# Oracle Design

## Overview

The oracle service is the critical bridge between chess platform APIs (Lichess, Chess.com) and the Smile4Money smart contracts. It monitors active matches, verifies game results, and triggers automatic payouts.

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Oracle Service                          │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │   Event      │  │   Result     │  │  Submission  │    │
│  │  Listener    │─▶│  Verifier    │─▶│   Engine     │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
│         │                  │                  │            │
└─────────┼──────────────────┼──────────────────┼────────────┘
          │                  │                  │
          ▼                  ▼                  ▼
┌─────────────────┐  ┌──────────────┐  ┌──────────────┐
│  Escrow         │  │  Lichess API │  │   Oracle     │
│  Contract       │  │  Chess.com   │  │  Contract    │
│  (Events)       │  │     API      │  │  (Submit)    │
└─────────────────┘  └──────────────┘  └──────────────┘
```

### Event Listener

Monitors the escrow contract for relevant events:

- **match.activated**: New match ready for monitoring
- **match.cancelled**: Stop monitoring cancelled matches
- **match.completed**: Remove from active monitoring

**Responsibilities:**
- Subscribe to Stellar event stream
- Parse event data to extract match details
- Add new matches to monitoring queue
- Remove completed/cancelled matches

### Result Verifier

Polls chess platform APIs to check game status:

- **Lichess API**: `GET /api/game/{gameId}`
- **Chess.com API**: `GET /pub/match/{matchId}`

**Responsibilities:**
- Poll APIs at regular intervals (e.g., every 30 seconds)
- Parse game status (ongoing, finished, aborted)
- Extract winner information (white, black, draw)
- Map platform results to contract Winner enum
- Verify game_id matches match record
- Handle API rate limits and errors

### Submission Engine

Submits verified results to blockchain:

1. Submit to oracle contract for record-keeping
2. Submit to escrow contract to trigger payout
3. Handle transaction failures and retries
4. Monitor transaction confirmation

**Responsibilities:**
- Sign and submit transactions
- Manage transaction fees
- Implement retry logic with exponential backoff
- Log all submissions for audit trail
- Alert on persistent failures

## Data Flow

### Match Monitoring Flow

```
1. Escrow emits "match.activated" event
   ├─> Event contains: match_id, game_id, platform
   └─> Oracle adds to monitoring queue

2. Oracle polls chess platform API
   ├─> GET /api/game/{game_id}
   ├─> Check status field
   └─> If status == "finished", proceed to verification

3. Oracle verifies result
   ├─> Extract winner from API response
   ├─> Map to Winner enum (Player1/Player2/Draw)
   ├─> Validate game_id matches match record
   └─> Prepare submission

4. Oracle submits to oracle contract
   ├─> Call oracle.submit_result(match_id, game_id, result)
   ├─> Wait for confirmation
   └─> Result stored on-chain

5. Oracle submits to escrow contract
   ├─> Call escrow.submit_result(match_id, winner, oracle_addr)
   ├─> Payout executes automatically
   ├─> Wait for confirmation
   └─> Remove from monitoring queue

6. Oracle logs completion
   └─> Record in database for audit trail
```

## API Integration

### Lichess API

**Endpoint:** `https://lichess.org/api/game/{gameId}`

**Authentication:** Bearer token (optional for public games)

**Response Format:**
```json
{
  "id": "abc123xyz",
  "status": "started" | "finished" | "aborted" | "timeout",
  "winner": "white" | "black" | null,
  "players": {
    "white": { "user": { "name": "player1" } },
    "black": { "user": { "name": "player2" } }
  }
}
```

**Rate Limits:**
- Authenticated: 60 requests/minute
- Unauthenticated: 20 requests/minute

**Result Mapping:**
```rust
match (status, winner) {
    ("finished", Some("white")) => MatchResult::Player1Wins,
    ("finished", Some("black")) => MatchResult::Player2Wins,
    ("finished", None) => MatchResult::Draw,
    ("aborted", _) => // Handle as cancellation
    ("timeout", _) => // Handle as forfeit
    _ => // Continue monitoring
}
```

### Chess.com API

**Endpoint:** `https://api.chess.com/pub/match/{matchId}`

**Authentication:** API key (optional)

**Response Format:**
```json
{
  "url": "https://www.chess.com/game/live/12345",
  "pgn": "...",
  "end_time": 1234567890,
  "white": { "username": "player1", "result": "win" | "loss" | "draw" },
  "black": { "username": "player2", "result": "win" | "loss" | "draw" }
}
```

**Rate Limits:**
- 300 requests/minute per IP

**Result Mapping:**
```rust
match (white.result, black.result) {
    ("win", "loss") => MatchResult::Player1Wins,
    ("loss", "win") => MatchResult::Player2Wins,
    ("draw", "draw") => MatchResult::Draw,
    _ => // Invalid state
}
```

## Security Considerations

### Oracle Trust Model

The oracle is a **trusted component** in the system. Players must trust that:

1. The oracle accurately fetches game results
2. The oracle does not collude with players
3. The oracle submits results promptly
4. The oracle's private key is secure

### Mitigation Strategies

#### 1. Multi-Oracle Consensus (Future)

Instead of a single oracle, use multiple independent oracles:

```
┌─────────┐  ┌─────────┐  ┌─────────┐
│ Oracle1 │  │ Oracle2 │  │ Oracle3 │
└────┬────┘  └────┬────┘  └────┬────┘
     │            │            │
     └────────────┼────────────┘
                  ▼
          ┌───────────────┐
          │  Consensus    │
          │  Contract     │
          └───────┬───────┘
                  │
                  ▼
          ┌───────────────┐
          │    Escrow     │
          │   Contract    │
          └───────────────┘
```

Require 2-of-3 or 3-of-5 oracle agreement before accepting result.

#### 2. Result Verification Window

Allow a challenge period after result submission:

```rust
pub fn submit_result_with_delay(
    env: Env,
    match_id: u64,
    winner: Winner,
) -> Result<(), Error> {
    // Store result but don't execute payout
    let result = PendingResult {
        winner,
        submitted_at: env.ledger().timestamp(),
        challenge_deadline: env.ledger().timestamp() + CHALLENGE_PERIOD,
    };
    env.storage().set(&DataKey::PendingResult(match_id), &result);
    Ok(())
}

pub fn execute_payout(env: Env, match_id: u64) -> Result<(), Error> {
    let result = get_pending_result(match_id)?;
    if env.ledger().timestamp() < result.challenge_deadline {
        return Err(Error::ChallengePeriodActive);
    }
    // Execute payout
    Ok(())
}
```

#### 3. On-Chain Result Storage

Store results in oracle contract before triggering payout:

- Provides audit trail
- Allows verification by third parties
- Enables dispute resolution
- Creates permanent record

#### 4. Oracle Rotation

Allow admin to update oracle address:

```rust
pub fn update_oracle(
    env: Env,
    new_oracle: Address,
) -> Result<(), Error> {
    let admin = get_admin(&env)?;
    admin.require_auth();
    env.storage().instance().set(&DataKey::Oracle, &new_oracle);
    Ok(())
}
```

#### 5. Monitoring and Alerts

Implement comprehensive monitoring:

- Alert on failed API calls
- Alert on transaction failures
- Alert on unusual result patterns
- Log all oracle actions
- Track response times

### Attack Vectors

#### 1. Oracle Compromise

**Attack:** Attacker gains control of oracle private key

**Impact:** Can submit false results and steal funds

**Mitigation:**
- Use hardware security module (HSM) for key storage
- Implement multi-signature oracle control
- Monitor for unusual submission patterns
- Implement emergency pause mechanism

#### 2. API Manipulation

**Attack:** Attacker manipulates chess platform API responses

**Impact:** Oracle submits incorrect results

**Mitigation:**
- Use HTTPS with certificate pinning
- Verify API responses with multiple calls
- Cross-check results from multiple sources
- Implement result sanity checks

#### 3. Front-Running

**Attack:** Attacker observes oracle transaction and front-runs with false result

**Impact:** False result accepted before legitimate one

**Mitigation:**
- Use private transaction submission
- Implement nonce-based ordering
- Check for existing results before submission

#### 4. Denial of Service

**Attack:** Attacker floods oracle with fake matches

**Impact:** Oracle overwhelmed, legitimate matches not processed

**Mitigation:**
- Rate limit match creation
- Require minimum stake amounts
- Implement priority queue based on stake size
- Monitor for spam patterns

## Implementation Considerations

### Technology Stack

**Recommended:**
- **Language**: Node.js (TypeScript) or Python
- **Blockchain SDK**: Stellar SDK (JS or Python)
- **Database**: PostgreSQL for audit logs
- **Queue**: Redis for match monitoring queue
- **Monitoring**: Prometheus + Grafana
- **Logging**: Winston (JS) or structlog (Python)

### Configuration

```typescript
interface OracleConfig {
  // Stellar configuration
  stellarNetwork: 'testnet' | 'mainnet';
  stellarRpcUrl: string;
  oracleSecretKey: string;
  
  // Contract addresses
  escrowContractId: string;
  oracleContractId: string;
  
  // API configuration
  lichessApiToken?: string;
  chessDotComApiKey?: string;
  
  // Polling configuration
  pollIntervalSeconds: number;
  maxConcurrentPolls: number;
  
  // Retry configuration
  maxRetries: number;
  retryBackoffMs: number;
  
  // Monitoring
  alertWebhookUrl?: string;
  logLevel: 'debug' | 'info' | 'warn' | 'error';
}
```

### Database Schema

```sql
CREATE TABLE matches (
  match_id BIGINT PRIMARY KEY,
  game_id VARCHAR(64) NOT NULL,
  platform VARCHAR(20) NOT NULL,
  player1 VARCHAR(56) NOT NULL,
  player2 VARCHAR(56) NOT NULL,
  stake_amount BIGINT NOT NULL,
  state VARCHAR(20) NOT NULL,
  created_at TIMESTAMP NOT NULL,
  activated_at TIMESTAMP,
  completed_at TIMESTAMP,
  result VARCHAR(20),
  INDEX idx_state (state),
  INDEX idx_game_id (game_id)
);

CREATE TABLE submissions (
  id SERIAL PRIMARY KEY,
  match_id BIGINT NOT NULL,
  submission_type VARCHAR(20) NOT NULL, -- 'oracle' or 'escrow'
  result VARCHAR(20) NOT NULL,
  transaction_hash VARCHAR(64) NOT NULL,
  submitted_at TIMESTAMP NOT NULL,
  confirmed_at TIMESTAMP,
  error TEXT,
  INDEX idx_match_id (match_id)
);

CREATE TABLE api_calls (
  id SERIAL PRIMARY KEY,
  match_id BIGINT NOT NULL,
  platform VARCHAR(20) NOT NULL,
  game_id VARCHAR(64) NOT NULL,
  response_status INT NOT NULL,
  response_body TEXT,
  called_at TIMESTAMP NOT NULL,
  INDEX idx_match_id (match_id)
);
```

### Error Handling

```typescript
class OracleService {
  async processMatch(matchId: number): Promise<void> {
    try {
      // Fetch match details
      const match = await this.getMatch(matchId);
      
      // Poll chess platform API
      const gameResult = await this.fetchGameResult(
        match.platform,
        match.gameId
      );
      
      if (!gameResult.isFinished) {
        // Game still in progress, continue monitoring
        return;
      }
      
      // Verify result
      const winner = this.mapResultToWinner(gameResult);
      
      // Submit to oracle contract
      await this.submitToOracleContract(matchId, match.gameId, winner);
      
      // Submit to escrow contract
      await this.submitToEscrowContract(matchId, winner);
      
      // Mark as completed
      await this.markMatchCompleted(matchId);
      
    } catch (error) {
      if (error instanceof ApiRateLimitError) {
        // Back off and retry later
        await this.scheduleRetry(matchId, 60000);
      } else if (error instanceof TransactionFailedError) {
        // Retry with higher fee
        await this.retryWithHigherFee(matchId);
      } else {
        // Log error and alert
        this.logger.error('Failed to process match', { matchId, error });
        await this.sendAlert(`Match ${matchId} processing failed: ${error.message}`);
      }
    }
  }
}
```

### Monitoring Metrics

Key metrics to track:

- **Match Processing Time**: Time from activation to payout
- **API Response Time**: Latency of chess platform APIs
- **Transaction Success Rate**: Percentage of successful submissions
- **Queue Depth**: Number of matches awaiting processing
- **Error Rate**: Failed API calls and transactions
- **Gas Costs**: Transaction fees over time

### Deployment

#### Development

```bash
# Install dependencies
npm install

# Configure environment
cp .env.example .env
# Edit .env with testnet credentials

# Run database migrations
npm run migrate

# Start oracle service
npm run dev
```

#### Production

```bash
# Build
npm run build

# Run with PM2 for auto-restart
pm2 start dist/index.js --name oracle-service

# Monitor logs
pm2 logs oracle-service

# Setup monitoring
pm2 install pm2-prometheus-exporter
```

## Future Enhancements

### 1. Multi-Oracle Consensus

Implement a consensus contract that requires multiple oracles to agree:

```rust
pub struct ConsensusContract;

impl ConsensusContract {
    pub fn submit_vote(
        env: Env,
        match_id: u64,
        winner: Winner,
        oracle: Address,
    ) -> Result<(), Error> {
        // Record oracle's vote
        // If threshold reached, trigger payout
    }
}
```

### 2. Automated Dispute Resolution

Allow players to challenge results with evidence:

```rust
pub fn challenge_result(
    env: Env,
    match_id: u64,
    evidence_url: String,
) -> Result<(), Error> {
    // Freeze payout
    // Notify admin for manual review
    // Require evidence submission
}
```

### 3. Real-Time WebSocket Integration

Use WebSocket APIs for instant result notification:

```typescript
// Lichess WebSocket
const ws = new WebSocket('wss://socket.lichess.org/watch/' + gameId);
ws.on('message', (data) => {
  if (data.status === 'finished') {
    processResult(gameId, data.winner);
  }
});
```

### 4. Decentralized Oracle Network

Integrate with existing oracle networks:

- Chainlink (if available on Stellar)
- Band Protocol
- Custom Stellar-native oracle network

### 5. Machine Learning Fraud Detection

Train models to detect suspicious patterns:

- Unusual betting patterns
- Rapid game completions
- Coordinated player behavior
- API response anomalies

## Testing Strategy

### Unit Tests

Test individual components:

```typescript
describe('ResultVerifier', () => {
  it('should map Lichess white win to Player1Wins', () => {
    const result = verifier.mapLichessResult({
      status: 'finished',
      winner: 'white'
    });
    expect(result).toBe(MatchResult.Player1Wins);
  });
});
```

### Integration Tests

Test API integration:

```typescript
describe('Lichess API Integration', () => {
  it('should fetch game result', async () => {
    const result = await lichessClient.getGame('abc123');
    expect(result).toHaveProperty('status');
    expect(result).toHaveProperty('winner');
  });
});
```

### End-to-End Tests

Test full flow on testnet:

```typescript
describe('Oracle E2E', () => {
  it('should process match from activation to payout', async () => {
    // Create match on testnet
    const matchId = await createTestMatch();
    
    // Wait for oracle to process
    await waitForCompletion(matchId, 60000);
    
    // Verify payout executed
    const match = await escrow.getMatch(matchId);
    expect(match.state).toBe(MatchState.Completed);
  });
});
```

## Operational Procedures

### Deployment Checklist

- [ ] Deploy oracle contract to testnet
- [ ] Deploy escrow contract to testnet
- [ ] Configure oracle service with testnet credentials
- [ ] Test with sample matches
- [ ] Monitor for 24 hours on testnet
- [ ] Deploy to mainnet
- [ ] Configure monitoring and alerts
- [ ] Document emergency procedures

### Emergency Procedures

#### Oracle Compromise

1. Immediately pause escrow contract
2. Rotate oracle private key
3. Update oracle address in escrow contract
4. Review recent submissions for fraud
5. Unpause contract after verification

#### API Outage

1. Monitor for API availability
2. Queue matches for retry
3. Alert admin if outage exceeds threshold
4. Consider manual result submission
5. Resume automatic processing when API recovers

#### Transaction Failures

1. Check Stellar network status
2. Increase transaction fee if network congested
3. Retry failed transactions
4. Alert if failures persist
5. Consider manual intervention

## Conclusion

The oracle is the critical trust component in Smile4Money. While the current design uses a single trusted oracle, the architecture supports future enhancements like multi-oracle consensus and decentralized verification. Proper implementation, monitoring, and security practices are essential for maintaining user trust and system reliability.
