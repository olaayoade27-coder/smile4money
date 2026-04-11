# smile4money — Competitive Chess Betting on Stellar

A trustless chess wagering platform built on Stellar Soroban smart contracts. Players stake XLM or USDC before a match, and the winner is automatically paid out the moment the game ends — no middleman, no delays, no trust required.

## 🎯 What is smile4money?

smile4money combines competitive chess with Stellar's fast settlement to create a fully on-chain betting platform for casual and high-stakes matches.

Players:

- Stake XLM or USDC into a Soroban escrow contract before a match begins
- Play their game on Lichess or Chess.com as normal
- Receive automatic payouts the instant the match result is verified on-chain

A custom Oracle bridges the Chess.com / Lichess API to the smart contract, verifying match results and triggering payouts without any manual intervention.

This makes smile4money:

✅ Trustless (no platform can withhold or delay winnings)  
✅ Transparent (all stakes and payouts are verifiable on-chain)  
✅ Instant (Stellar's fast finality means payouts settle in seconds)  
✅ Accessible (anyone with a Stellar wallet can participate)

## 🚀 Features

- **Create a Match**: Set stake amount, currency (XLM or USDC), and link a Lichess/Chess.com game ID
- **Escrow Stakes**: Both players deposit funds into the contract before the game starts
- **Oracle Integration**: Real-time result verification via Lichess/Chess.com APIs
- **Automatic Payouts**: Winner receives the full pot the moment the result is confirmed
- **Draw Handling**: Stakes are returned to both players in the event of a draw
- **Transparent**: All escrow balances and payout history are verifiable on-chain

## 🛠️ Quick Start

### Prerequisites

- Rust (1.70+)
- Soroban CLI
- Stellar CLI

### Build

```bash
./scripts/build.sh
```

### Test

```bash
./scripts/test.sh
```

### Setup Environment

Copy the example environment file:

```bash
cp .env.example .env
```

Configure your environment variables in `.env`:

```env
# Network configuration
STELLAR_NETWORK=testnet
STELLAR_RPC_URL=https://soroban-testnet.stellar.org

# Contract addresses (after deployment)
CONTRACT_ESCROW=<your-contract-id>
CONTRACT_ORACLE=<your-contract-id>

# Oracle configuration
LICHESS_API_TOKEN=<your-lichess-api-token>
CHESSDOTCOM_API_KEY=<your-chessdotcom-api-key>

# Frontend configuration
VITE_STELLAR_NETWORK=testnet
VITE_STELLAR_RPC_URL=https://soroban-testnet.stellar.org
```

Network configurations are defined in `environments.toml`:

- `testnet` — Stellar testnet
- `mainnet` — Stellar mainnet
- `futurenet` — Stellar futurenet
- `standalone` — Local development

### Deploy to Testnet

```bash
# Configure your testnet identity first
stellar keys generate deployer --network testnet

# Deploy
./scripts/deploy_testnet.sh
```

## 📖 Documentation

- [Architecture Overview](docs/architecture.md)
- [Oracle Design](docs/oracle.md)
- [Threat Model & Security](docs/security.md)
- [Roadmap](docs/roadmap.md)

## 🎓 Smart Contract API

### Match Management

```
create_match(stake_amount, token, game_id, platform) -> u64
get_match(match_id) -> Match
cancel_match(match_id)
```

### Escrow

```
deposit(match_id)
get_escrow_balance(match_id) -> i128
is_funded(match_id) -> bool
```

### Oracle & Payouts

```
submit_result(match_id, winner)
verify_result(match_id) -> bool
execute_payout(match_id)
```

## 🧪 Testing

Run tests:

```bash
cargo test
```

## 🌍 Why This Matters

**The Problem**: Current chess betting and tournament prize payouts are slow and rely entirely on the platform's honesty. Players have no guarantee their winnings will be paid out fairly or on time.

**The Solution**: By holding stakes in a Soroban smart contract and automating payouts via a verified Oracle, smile4money removes the need to trust any third party.

## 🗺️ Roadmap

- **v1.0 (Current)**: XLM-only escrow, Lichess Oracle integration, basic match flow
- **v1.1**: USDC and custom token support, Chess.com Oracle
- **v2.0**: Multi-game tournaments, bracket payouts
- **v3.0**: Frontend UI with wallet integration
- **v4.0**: Mobile app, ELO-based matchmaking, leaderboards

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Stellar Development Foundation](https://stellar.org) for Soroban
- [Lichess](https://lichess.org) for their open API
- [Chess.com](https://chess.com) for their developer platform
