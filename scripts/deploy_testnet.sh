#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# deploy_testnet.sh — Build and deploy smile4money contracts to Stellar testnet
#
# Prerequisites:
#   stellar keys generate deployer --network testnet
#
# Usage:
#   ./scripts/deploy_testnet.sh
#
# Writes CONTRACT_ESCROW and CONTRACT_ORACLE to .env on success.
# ---------------------------------------------------------------------------

NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
IDENTITY="deployer"
WASM_DIR="target/wasm32-unknown-unknown/release"

# Verify stellar CLI is available
if ! command -v stellar &>/dev/null; then
  echo "Error: stellar CLI not found. Install it from https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli" >&2
  exit 1
fi

# Verify the deployer identity exists
if ! stellar keys show "$IDENTITY" &>/dev/null; then
  echo "Error: identity '$IDENTITY' not found. Run: stellar keys generate $IDENTITY --network $NETWORK" >&2
  exit 1
fi

DEPLOYER_ADDRESS=$(stellar keys address "$IDENTITY")
echo "Deployer: $DEPLOYER_ADDRESS"

# Fund account via friendbot if needed
echo "Funding deployer account via friendbot..."
curl -sf "https://friendbot.stellar.org?addr=${DEPLOYER_ADDRESS}" -o /dev/null || true

# Build WASM
echo "Building contracts..."
cargo build --target wasm32-unknown-unknown --release --quiet

ESCROW_WASM="$WASM_DIR/escrow.wasm"
ORACLE_WASM="$WASM_DIR/oracle.wasm"

if [[ ! -f "$ESCROW_WASM" ]]; then
  echo "Error: $ESCROW_WASM not found after build" >&2
  exit 1
fi
if [[ ! -f "$ORACLE_WASM" ]]; then
  echo "Error: $ORACLE_WASM not found after build" >&2
  exit 1
fi

# Deploy escrow contract
echo "Deploying escrow contract..."
CONTRACT_ESCROW=$(stellar contract deploy \
  --wasm "$ESCROW_WASM" \
  --source "$IDENTITY" \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")
echo "Escrow contract: $CONTRACT_ESCROW"

# Deploy oracle contract
echo "Deploying oracle contract..."
CONTRACT_ORACLE=$(stellar contract deploy \
  --wasm "$ORACLE_WASM" \
  --source "$IDENTITY" \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")
echo "Oracle contract: $CONTRACT_ORACLE"

# Initialize oracle contract (admin = deployer)
echo "Initializing oracle contract..."
stellar contract invoke \
  --id "$CONTRACT_ORACLE" \
  --source "$IDENTITY" \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --admin "$DEPLOYER_ADDRESS"

# Initialize escrow contract (oracle = oracle contract address, admin = deployer)
echo "Initializing escrow contract..."
stellar contract invoke \
  --id "$CONTRACT_ESCROW" \
  --source "$IDENTITY" \
  --network "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --oracle "$CONTRACT_ORACLE" \
  --admin "$DEPLOYER_ADDRESS"

# Write contract IDs to .env
ENV_FILE=".env"
if [[ ! -f "$ENV_FILE" ]]; then
  cp .env.example "$ENV_FILE"
fi

# Update or append CONTRACT_ESCROW
if grep -q "^CONTRACT_ESCROW=" "$ENV_FILE"; then
  sed -i "s|^CONTRACT_ESCROW=.*|CONTRACT_ESCROW=$CONTRACT_ESCROW|" "$ENV_FILE"
else
  echo "CONTRACT_ESCROW=$CONTRACT_ESCROW" >> "$ENV_FILE"
fi

# Update or append CONTRACT_ORACLE
if grep -q "^CONTRACT_ORACLE=" "$ENV_FILE"; then
  sed -i "s|^CONTRACT_ORACLE=.*|CONTRACT_ORACLE=$CONTRACT_ORACLE|" "$ENV_FILE"
else
  echo "CONTRACT_ORACLE=$CONTRACT_ORACLE" >> "$ENV_FILE"
fi

echo ""
echo "Deployment complete."
echo "  Escrow:  $CONTRACT_ESCROW"
echo "  Oracle:  $CONTRACT_ORACLE"
echo "Contract IDs written to $ENV_FILE"
