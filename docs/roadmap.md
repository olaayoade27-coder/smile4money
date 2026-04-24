# Roadmap

## v1.0 — Current

Core escrow and oracle functionality on Stellar Soroban.

- XLM escrow contract with full match lifecycle (Pending → Active → Completed / Cancelled)
- Lichess oracle integration
- Admin pause / unpause circuit breaker
- Re-initialization guard on both contracts
- TTL extension on all persistent storage entries
- On-chain events for all state transitions
- GitHub Actions CI (cargo test + cargo clippy)

## v1.1 — Token Support & Chess.com Oracle

- USDC and arbitrary SAC token support (token address is already a parameter; this milestone validates multi-token flows end-to-end)
- Chess.com oracle integration
- `update_oracle` admin function for key rotation without redeployment
- Game ID uniqueness enforcement to prevent duplicate match payouts

## v2.0 — Tournaments

- Multi-match tournament bracket contract
- Bracket payout logic (winner advances, loser is eliminated and refunded)
- Tournament admin role with configurable prize splits

## v3.0 — Frontend

- Web frontend with Stellar wallet integration (Freighter / Albedo)
- Match creation and deposit UI
- Live match status and payout history
- Oracle status dashboard

## v4.0 — Mobile & Matchmaking

- Mobile app (iOS / Android)
- ELO-based matchmaking — players are paired with opponents of similar rating
- Global leaderboard with on-chain verifiable win/loss records
- Configurable stake tiers and time controls
