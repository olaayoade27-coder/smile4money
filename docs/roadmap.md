# Roadmap

## v1.0 — Current

- XLM escrow via Soroban smart contract
- Lichess Oracle integration
- Full match lifecycle: create → deposit → result → payout / cancel
- Admin controls: pause / unpause / oracle rotation
- On-chain event emission for all state transitions
- GitHub Actions CI (test + WASM build)

## v1.1

- USDC and arbitrary SEP-41 token support (already token-agnostic in contract; needs oracle + frontend wiring)
- Chess.com Oracle support
- Timeout-based escape hatch: player can cancel an `Active` match after N ledgers if no result is submitted (oracle liveness protection)
- Oracle service open-sourced and documented

## v2.0

- Multi-game tournaments with bracket payouts
- Decentralised oracle: multiple oracle addresses with threshold agreement
- Platform fee mechanism (configurable basis points, sent to treasury address)

## v3.0

- Frontend UI with Freighter / Lobstr wallet integration
- Match discovery feed
- Player profiles and match history

## v4.0

- Mobile app (React Native)
- ELO-based matchmaking
- Leaderboards and seasonal rankings
