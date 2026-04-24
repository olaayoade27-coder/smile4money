# Security & Threat Model

## Overview

This document analyzes the security properties, trust assumptions, attack vectors, and mitigation strategies for the Smile4Money platform.

## Trust Model

### What Users Must Trust

1. **Smart Contract Code**
   - Escrow logic correctly holds and distributes funds
   - No bugs that allow unauthorized fund withdrawal
   - State transitions follow documented rules
   - Admin cannot steal funds (only pause/unpause)

2. **Oracle Service**
   - Accurately fetches game results from chess platforms
   - Does not collude with players to submit false results
   - Submits results promptly after game completion
   - Private key is securely stored

3. **Chess Platforms**
   - Lichess/Chess.com APIs provide accurate game results
   - Game results cannot be manipulated by players
   - APIs remain available and reliable

4. **Stellar Network**
   - Transactions are processed correctly
   - Network remains available
   - Consensus is not compromised

### What Users Don't Need to Trust

1. **Platform Operators**
   - Cannot withhold or delay payouts (automatic via smart contract)
   - Cannot modify match results (oracle-verified)
   - Cannot access escrowed funds (held by contract)

2. **Other Players**
   - Cannot withdraw opponent's funds
   - Cannot cancel match after both deposited
   - Cannot manipulate game results (verified by oracle)

## Threat Analysis

### Critical Threats (High Impact, High Likelihood)

#### 1. Oracle Private Key Compromise

**Attack:** Attacker gains access to oracle's private key

**Impact:**
- Can submit false results for any match
- Can steal all escrowed funds by declaring attacker as winner
- Complete loss of system integrity

**Likelihood:** Medium (depends on key management practices)

**Mitigation:**
- Use hardware security module (HSM) for key storage
- Implement multi-signature oracle control (2-of-3 or 3-of-5)
- Monitor for unusual submission patterns
- Implement rate limiting on result submissions
- Add challenge period before payout execution
- Regular security audits of oracle infrastructure

**Status:** ⚠️ Partially mitigated (single oracle, no HSM requirement)

---

#### 2. Smart Contract Bugs

**Attack:** Exploit bugs in escrow or oracle contracts

**Impact:**
- Unauthorized fund withdrawal
- Denial of service
- State corruption
- Loss of user funds

**Likelihood:** Medium (complex contract logic)

**Mitigation:**
- Comprehensive test coverage (unit, integration, E2E)
- Formal verification of critical functions
- External security audit before mainnet deployment
- Bug bounty program
- Gradual rollout with stake limits
- Emergency pause mechanism

**Status:** ⚠️ Partially mitigated (see [issues.md](../issues.md) for known bugs)

---

#### 3. Oracle-Player Collusion

**Attack:** Oracle colludes with a player to submit false result

**Impact:**
- Colluding player wins despite losing game
- Honest player loses funds
- System reputation damage

**Likelihood:** Low (requires oracle compromise)

**Mitigation:**
- Multi-oracle consensus (require 2-of-3 agreement)
- On-chain result storage for audit trail
- Challenge period allowing dispute
- Reputation system for oracles
- Slashing for proven false submissions

**Status:** ❌ Not mitigated (single oracle, no challenge period)

---

### High Threats (High Impact, Low Likelihood)

#### 4. Stellar Network Compromise

**Attack:** Stellar consensus is compromised

**Impact:**
- Transactions can be censored or reordered
- Double-spend attacks possible
- Complete system failure

**Likelihood:** Very Low (requires compromising Stellar validators)

**Mitigation:**
- Inherent to blockchain choice
- Monitor Stellar network health
- Have contingency plan for network issues

**Status:** ✅ Accepted risk (trust in Stellar network)

---

#### 5. Chess Platform API Compromise

**Attack:** Attacker manipulates Lichess/Chess.com API responses

**Impact:**
- Oracle receives false game results
- Incorrect payouts executed

**Likelihood:** Very Low (requires compromising chess platform)

**Mitigation:**
- Use HTTPS with certificate pinning
- Cross-check results from multiple sources
- Verify game exists on platform website
- Implement sanity checks (e.g., game duration)

**Status:** ⚠️ Partially mitigated (HTTPS, but no cross-checking)

---

### Medium Threats (Medium Impact, Medium Likelihood)

#### 6. Front-Running Attacks

**Attack:** Attacker observes oracle transaction and front-runs with false result

**Impact:**
- False result accepted before legitimate one
- Incorrect payout

**Likelihood:** Low (requires monitoring mempool and fast submission)

**Mitigation:**
- Check for existing results before submission
- Use private transaction submission
- Implement nonce-based ordering
- First submission wins (duplicate prevention)

**Status:** ✅ Mitigated (duplicate submission prevention)

---

#### 7. Denial of Service on Oracle

**Attack:** Flood oracle with fake matches or API requests

**Impact:**
- Oracle overwhelmed, legitimate matches not processed
- Delayed payouts
- Increased operational costs

**Likelihood:** Medium (easy to execute)

**Mitigation:**
- Rate limit match creation
- Require minimum stake amounts
- Implement priority queue based on stake size
- Auto-scaling infrastructure
- DDoS protection (Cloudflare, etc.)

**Status:** ⚠️ Partially mitigated (minimum stake check, no rate limiting)

---

#### 8. Reentrancy Attacks

**Attack:** Malicious token contract calls back into escrow during transfer

**Impact:**
- Double withdrawal
- State corruption
- Fund theft

**Likelihood:** Low (Soroban execution model prevents reentrancy)

**Mitigation:**
- Soroban's execution model is reentrancy-safe
- Use checks-effects-interactions pattern
- Update state before external calls

**Status:** ✅ Mitigated (Soroban architecture)

---

### Low Threats (Low Impact or Very Low Likelihood)

#### 9. Integer Overflow

**Attack:** Cause integer overflow in stake calculations

**Impact:**
- Incorrect payout amounts
- Potential fund loss

**Likelihood:** Very Low (checked arithmetic used)

**Mitigation:**
- Use checked arithmetic (`checked_add`, etc.)
- Overflow checks enabled in release mode
- Test with extreme values

**Status:** ✅ Mitigated (checked arithmetic, overflow checks enabled)

---

#### 10. Storage Expiration

**Attack:** Wait for match TTL to expire, causing data loss

**Impact:**
- Match data unavailable
- Cannot complete payout
- Funds locked

**Likelihood:** Low (30-day TTL, extended on every write)

**Mitigation:**
- Extend TTL on every storage write
- Set TTL to 30 days (518,400 ledgers)
- Monitor for expiring entries
- Oracle processes matches promptly

**Status:** ✅ Mitigated (TTL extension implemented)

---

#### 11. Unauthorized Cancellation

**Attack:** Player1 cancels match after player2 deposits

**Impact:**
- Player2 inconvenienced (receives refund)
- Griefing attack

**Likelihood:** Medium (easy to execute)

**Mitigation:**
- Allow either player to cancel Pending matches
- Automatic refunds ensure no fund loss
- Consider requiring both players to approve cancellation after deposits

**Status:** ⚠️ Partially mitigated (refunds work, but griefing possible)

---

## Known Vulnerabilities

See [issues.md](../issues.md) for a comprehensive list of 34 known issues including:

### Critical Issues
- Multiple initialization vulnerabilities (escrow and oracle)
- No mechanism to update oracle address
- No admin role for emergency controls
- Oracle-escrow integration issues
- Missing TTL extensions (now fixed)
- Authorization issues in cancellation

### High Priority Issues
- Zero stake amount allowed (now fixed)
- Player2 cannot cancel matches (now fixed)
- Missing validation for winner/game_id
- No events for key operations (now fixed)
- Persistent storage TTL issues (now fixed)

### Medium Priority Issues
- Self-match allowed
- Game ID uniqueness not enforced
- Missing state checks in deposit
- Boolean arithmetic in balance calculation

## Security Best Practices

### For Developers

1. **Code Review**
   - All changes require peer review
   - Security-focused review for critical functions
   - Test coverage required for all new code

2. **Testing**
   - Unit tests for all functions
   - Integration tests for contract interactions
   - Fuzz testing for input validation
   - Property-based testing for invariants

3. **Auditing**
   - External security audit before mainnet
   - Regular audits after major changes
   - Bug bounty program for responsible disclosure

4. **Monitoring**
   - Real-time monitoring of contract events
   - Alert on unusual patterns
   - Track all oracle submissions
   - Monitor gas costs and network health

### For Oracle Operators

1. **Key Management**
   - Use HSM for private key storage
   - Never expose private key in logs or code
   - Rotate keys periodically
   - Use multi-signature control

2. **Infrastructure Security**
   - Run oracle service in isolated environment
   - Use firewall to restrict access
   - Enable DDoS protection
   - Regular security updates

3. **Operational Security**
   - Monitor for unusual submission patterns
   - Alert on failed transactions
   - Log all actions for audit trail
   - Have incident response plan

4. **API Security**
   - Use HTTPS with certificate pinning
   - Rotate API keys regularly
   - Monitor for API anomalies
   - Have backup API sources

### For Users

1. **Wallet Security**
   - Use hardware wallet for large stakes
   - Never share private keys
   - Verify contract addresses before transactions
   - Use reputable wallet software

2. **Match Verification**
   - Verify game_id matches your chess game
   - Check opponent address before depositing
   - Confirm stake amount is correct
   - Wait for both deposits before starting game

3. **Risk Management**
   - Start with small stakes
   - Understand the trust model
   - Know that oracle can submit false results
   - Be aware of smart contract risks

## Incident Response

### Security Incident Procedure

1. **Detection**
   - Automated monitoring alerts
   - User reports
   - Security researcher disclosure

2. **Assessment**
   - Determine severity and impact
   - Identify affected matches/users
   - Estimate potential losses

3. **Containment**
   - Pause contract if necessary
   - Stop oracle service if compromised
   - Prevent further damage

4. **Investigation**
   - Analyze attack vector
   - Identify root cause
   - Determine extent of compromise

5. **Remediation**
   - Deploy fix if possible
   - Rotate compromised keys
   - Refund affected users if necessary

6. **Communication**
   - Notify affected users
   - Public disclosure of incident
   - Transparency about losses and remediation

7. **Post-Mortem**
   - Document incident timeline
   - Identify lessons learned
   - Implement preventive measures

### Emergency Contacts

- **Admin Address**: Can pause/unpause contract
- **Oracle Operator**: Can stop result submissions
- **Security Team**: security@smile4money.io (example)
- **Bug Bounty**: bugbounty@smile4money.io (example)

## Audit Checklist

Before mainnet deployment, ensure:

- [ ] All known bugs from issues.md are fixed
- [ ] External security audit completed
- [ ] Test coverage > 90%
- [ ] Fuzz testing performed
- [ ] Oracle infrastructure secured (HSM, firewall, monitoring)
- [ ] Emergency procedures documented
- [ ] Bug bounty program launched
- [ ] Monitoring and alerting configured
- [ ] Incident response plan tested
- [ ] User documentation includes security warnings
- [ ] Gradual rollout plan with stake limits
- [ ] Multi-signature admin control implemented

## Responsible Disclosure

If you discover a security vulnerability:

1. **Do Not** exploit it or disclose publicly
2. **Do** email security@smile4money.io with details
3. **Include** steps to reproduce, impact assessment, and suggested fix
4. **Wait** for response (we aim for 48 hours)
5. **Coordinate** disclosure timeline with team

We offer rewards for responsible disclosure:

- **Critical**: Up to $10,000
- **High**: Up to $5,000
- **Medium**: Up to $1,000
- **Low**: Up to $500

## Security Roadmap

### Phase 1: Current (Testnet)
- Single oracle model
- Basic authorization checks
- Emergency pause mechanism
- Event emission for monitoring

### Phase 2: Enhanced Security (Pre-Mainnet)
- Fix all known bugs from issues.md
- External security audit
- Bug bounty program
- Comprehensive monitoring
- HSM for oracle key

### Phase 3: Decentralization (Post-Mainnet)
- Multi-oracle consensus (2-of-3)
- Challenge period for disputes
- Decentralized oracle network
- Governance for parameter updates

### Phase 4: Advanced Features
- Zero-knowledge proofs for privacy
- Cross-chain support
- Automated dispute resolution
- Insurance fund for oracle failures

## Conclusion

Smile4Money's security model relies heavily on the trusted oracle. While the smart contracts provide trustless escrow and automatic payouts, the oracle remains a centralized point of failure. Future enhancements should focus on decentralizing the oracle through multi-oracle consensus and challenge mechanisms.

Users should understand the trust assumptions and start with small stakes until the system has proven reliability. Operators should follow security best practices and maintain comprehensive monitoring.

The known issues documented in issues.md must be addressed before mainnet deployment. A professional security audit is essential to identify additional vulnerabilities and validate the security model.
