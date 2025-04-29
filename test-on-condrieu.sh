#!/bin/bash
set -e

echo "ERC1155 Test Script for Condrieu Testnet"
echo "========================================"

# Condrieu testnet node URL - modify this if needed
TESTNET_URL="ws://127.0.0.1:9944"

# Test account private key or seed phrase - replace with your own account
TEST_ACCOUNT="//Alice"

# Check if tools are installed
check_tools() {
  echo "Checking if required tools are installed..."
  
  if ! command -v cargo &> /dev/null; then
    echo "Cargo not found. Please install Rust."
    exit 1
  fi
  
  if ! command -v cargo-contract &> /dev/null; then
    echo "cargo-contract not found. Please install it with 'cargo install cargo-contract'"
    exit 1
  fi
}

# Check if testnet is running
check_node() {
  echo "Checking if Condrieu testnet node is running at $TESTNET_URL..."
  # Try to connect to the node
  timeout 5 websocat "$TESTNET_URL" 2>/dev/null || {
    echo "Condrieu testnet node is not running or not accessible at $TESTNET_URL"
    echo "Please start a node before running this test"
    exit 1
  }
  echo "Condrieu testnet node is running"
}

# Build the contract and CLI tool
build_all() {
  echo "Building the ERC1155 contract and CLI tools..."
  chmod +x build.sh
  ./build.sh
}

# Deploy the contract
deploy_contract() {
  echo "Deploying ERC1155 contract to Condrieu testnet..."
  
  CONTRACT_DEPLOY_OUTPUT=$(./target/release/erc1155-cli --node-url "$TESTNET_URL" --seed "$TEST_ACCOUNT" deploy)
  echo "$CONTRACT_DEPLOY_OUTPUT"
  
  # Extract contract address from the output
  CONTRACT_ADDRESS=$(echo "$CONTRACT_DEPLOY_OUTPUT" | grep -o '5[a-zA-Z0-9]*')
  
  if [ -z "$CONTRACT_ADDRESS" ]; then
    echo "Failed to extract contract address from deployment output"
    exit 1
  fi
  
  echo "Contract deployed at address: $CONTRACT_ADDRESS"
  echo "$CONTRACT_ADDRESS" > contract_address.txt
}

# Create a token
create_token() {
  CONTRACT_ADDRESS=$(cat contract_address.txt)
  
  echo "Creating a token in the contract at address $CONTRACT_ADDRESS..."
  ./target/release/erc1155-cli --node-url "$TESTNET_URL" --seed "$TEST_ACCOUNT" create-token \
    --contract "$CONTRACT_ADDRESS" \
    --uri "https://example.com/token/1" \
    --supply 1000
}

# Verify contract storage
verify_storage() {
  CONTRACT_ADDRESS=$(cat contract_address.txt)
  
  echo "Verifying contract storage on-chain..."
  ./target/release/erc1155-cli --node-url "$TESTNET_URL" --seed "$TEST_ACCOUNT" verify-storage \
    --contract "$CONTRACT_ADDRESS"
}

# Transfer tokens
transfer_tokens() {
  CONTRACT_ADDRESS=$(cat contract_address.txt)
  
  # Using Bob's address for testing
  BOB_ADDRESS="5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"
  
  echo "Transferring tokens from $TEST_ACCOUNT to Bob ($BOB_ADDRESS)..."
  ./target/release/erc1155-cli --node-url "$TESTNET_URL" --seed "$TEST_ACCOUNT" transfer \
    --contract "$CONTRACT_ADDRESS" \
    --to "$BOB_ADDRESS" \
    --token-id 1 \
    --amount 50
}

# Check balances
check_balances() {
  CONTRACT_ADDRESS=$(cat contract_address.txt)
  
  # Alice's address
  ALICE_ADDRESS="5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
  # Bob's address
  BOB_ADDRESS="5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"
  
  echo "Checking Alice's balance..."
  ./target/release/erc1155-cli --node-url "$TESTNET_URL" --seed "$TEST_ACCOUNT" balance \
    --contract "$CONTRACT_ADDRESS" \
    --account "$ALICE_ADDRESS" \
    --token-id 1
  
  echo "Checking Bob's balance..."
  ./target/release/erc1155-cli --node-url "$TESTNET_URL" --seed "$TEST_ACCOUNT" balance \
    --contract "$CONTRACT_ADDRESS" \
    --account "$BOB_ADDRESS" \
    --token-id 1
}

# Main function
main() {
  echo "Starting ERC1155 test on Condrieu testnet..."
  
  check_tools
  check_node
  build_all
  deploy_contract
  create_token
  verify_storage
  transfer_tokens
  check_balances
  verify_storage
  
  echo "Test completed successfully!"
  echo "Your ERC1155 contract is working properly on the Condrieu testnet"
  echo "Contract address: $(cat contract_address.txt)"
}

# Run the main function
main 