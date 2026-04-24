# Roadmap

## v1.0 — Foundation (Current)

Core escrow and oracle contracts on Stellar Soroban.

**Smart Contracts**
- Escrow contract: match lifecycle (Pending → Active → Completed/Cancelled)
- Oracle contract: on-chain result registry
- XLM-only staking
- Lichess game ID integration
- Admin pause/unpause controls
- TTL-protected persistent storage
- Full event emission for all state changes
- Overflow-safe match counter

**Testing**
- Unit tests for all escrow and oracle functions
- Auth, state machine, payout, and event tests
- GitHub Actions CI (cargo test + cargo clippy)

---

## v1.1 — Token Support & Security Hardening

**Token Support**
- USDC staking (Stellar USDC via Circle)
- Custom token support (any Soroban token contract)

**Chess.com Integration**
- Chess.com oracle adapter
- API key management for Chess.com developer platform

**Security Fixes**
- Self-match guard: reject `player1 == player2` in `create_match`
- Game ID uniqueness: reject duplicate `game_id` across matches
- Oracle rotation: `update_oracle(new_oracle)` admin function
- Timeout refund: allow players to claim refund after configurable ledger timeout if oracle has not submitted a result

---

## v2.0 — Tournaments

**Multi-game Tournaments**
- Tournament contract: bracket or round-robin format
- Automatic bracket progression on match completion
- Prize pool distribution: winner-takes-all or tiered payouts

**Escrow Improvements**
- Multi-match escrow: single deposit covers an entire tournament entry
- Partial refunds for early exits

---

## v3.0 — Frontend

**Web Application**
- Stellar wallet integration (Freighter, xBull, Lobstr)
- Match creation UI: set stake, token, game ID, platform
- Live match status: deposit tracking, result feed
- Payout history and on-chain transaction explorer links
- Testnet and mainnet support

---

## v4.0 — Mobile & Matchmaking

**Mobile App**
- iOS and Android clients
- Push notifications for match events (deposit received, result submitted, payout sent)

**ELO-based Matchmaking**
- On-chain ELO rating system
- Matchmaking contract: pair players by rating range and stake preference
- Leaderboards: top players by winnings, win rate, and ELO

---

## Backlog / Under Consideration

- **Decentralized oracle**: Replace the single trusted oracle with a threshold multi-sig oracle or integration with a decentralized oracle network
- **Spectator staking**: Allow third parties to bet on match outcomes (prediction market style)
- **Dispute resolution**: On-chain dispute mechanism for contested results
- **Cross-chain**: Bridge to other networks (e.g., Ethereum, Solana) for broader player access
