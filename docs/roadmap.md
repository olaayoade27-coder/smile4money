# Roadmap

## v1.0 — Current

Core escrow and oracle functionality on Stellar Soroban.

- XLM-only escrow contract
- Lichess Oracle integration
- Basic match flow: create → deposit → result → payout
- Draw handling (stakes returned to both players)
- Admin pause / unpause controls
- On-chain events for all state transitions
- GitHub Actions CI (cargo test + cargo build --target wasm32)

## v1.1 — Multi-token & Chess.com

- USDC support (and any SEP-41 token)
- Chess.com Oracle integration
- Oracle address rotation (`update_oracle` admin function)
- Timeout-based cancellation: player2 can cancel after a configurable ledger timeout if player1 never deposits

## v2.0 — Tournaments

- Multi-game tournament brackets
- Bracket payout contract: distributes prize pool across rounds
- Tournament admin: create bracket, seed players, advance rounds
- On-chain bracket state and results

## v3.0 — Frontend

- Web UI with Stellar wallet integration (Freighter, xBull)
- Match creation and deposit flow in-browser
- Live match status and payout history
- Off-chain indexer for event streaming

## v4.0 — Mobile & Matchmaking

- Mobile app (iOS / Android)
- ELO-based matchmaking: pair players of similar rating
- Leaderboards: on-chain ranking by winnings and win rate
- Push notifications for match events
