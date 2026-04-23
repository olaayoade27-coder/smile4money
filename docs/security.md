# Threat Model & Security

This document describes the security properties of smile4money, known trust assumptions, and mitigations in place.

## Trust Assumptions

| Actor | Trusted for | Mitigation if compromised |
|---|---|---|
| Oracle service | Submitting correct game results | Admin can rotate oracle address via `update_oracle` |
| Admin address | Pausing contract, rotating oracle | Admin key should be a multisig or hardware wallet |
| Stellar network | Transaction finality and ordering | Inherent to the platform; no mitigation needed |
| Token contract | Correct SEP-41 transfer behaviour | Use only audited, well-known token contracts |

## Access Control

### Escrow Contract

| Function | Who can call |
|---|---|
| `initialize` | Anyone (once only — panics on second call) |
| `create_match` | `player1` (requires auth) |
| `deposit` | `player1` or `player2` (requires auth) |
| `submit_result` | Registered oracle address only |
| `cancel_match` | `player1` or `player2` (requires auth) |
| `pause` / `unpause` | Admin only |

### Oracle Contract

| Function | Who can call |
|---|---|
| `initialize` | Anyone (once only — panics on second call) |
| `submit_result` | Admin (oracle service) only |
| `get_result` / `has_result` | Anyone (read-only) |

## Known Risks & Mitigations

### Oracle Compromise

**Risk**: A compromised oracle can submit incorrect results and redirect payouts.

**Mitigations**:
- All oracle submissions emit on-chain events, making manipulation auditable.
- Admin can rotate the oracle address without redeploying.
- Oracle contract records all results independently for cross-referencing.

### Re-initialization

**Risk**: An attacker calls `initialize` again to overwrite the oracle or admin address.

**Mitigation**: Both contracts check for existing storage keys on `initialize` and panic if already set.

### Self-match

**Risk**: A single address creates a match against itself to cycle funds.

**Mitigation**: `create_match` rejects `player1 == player2` with `Error::InvalidPlayers`.

**Note**: `InvalidPlayers` is documented as a planned guard — verify it is enforced in the current contract version.

### Duplicate Game ID

**Risk**: The same chess game ID is used across multiple matches, allowing the oracle to pay out multiple times for one game.

**Mitigation**: `create_match` tracks used `game_id` values and rejects duplicates.

**Note**: Verify `DataKey::GameId` uniqueness tracking is implemented in the current version.

### Zero-stake Match

**Risk**: Matches created with `stake_amount = 0` waste ledger storage.

**Mitigation**: `create_match` returns `Error::InvalidAmount` if `stake_amount <= 0`.

### Storage Expiry

**Risk**: Soroban persistent storage entries expire after their TTL. Expired match records cause `MatchNotFound` errors for active matches.

**Mitigation**: Every persistent write calls `extend_ttl` with `MATCH_TTL_LEDGERS` (~30 days). The oracle service should re-extend TTL for long-running matches.

### Player2 Fund Lock

**Risk**: If player1 abandons a match after player2 has deposited, player2's funds are locked.

**Mitigation**: Either player can cancel a `Pending` match. Player2 can cancel and recover their deposit at any time before the match becomes `Active`.

### Integer Overflow

**Risk**: `MatchCount` wraps silently in release mode.

**Mitigation**: `checked_add(1)` is used when incrementing `MatchCount`, returning `Error::Overflow` on overflow.

### Contract Pause

**Risk**: A critical vulnerability is discovered post-deployment.

**Mitigation**: Admin can call `pause()` to block `create_match`, `deposit`, and `submit_result`. Existing completed matches are unaffected.

## What Is Not Covered

- **Frontend security** — The frontend is out of scope for v1.0. Wallet interactions should follow Stellar wallet security best practices.
- **Oracle service key management** — The oracle private key must be stored securely (e.g., HSM or secrets manager). This is an operational concern.
- **Stellar network-level attacks** — Sequence number manipulation, fee bumping, and similar Stellar-level concerns are outside the scope of the smart contracts.

## Reporting Vulnerabilities

If you discover a security issue, please open a private GitHub issue or contact the maintainers directly before public disclosure.
