# Testing ERC1155 on Condrieu Testnet

This guide provides step-by-step instructions for testing your ERC1155 smart contract on the Condrieu testnet, including local storage with SLED database and price-triggered automatic execution.

## Features

1. **On-chain Storage** - Store and verify ERC1155 contract data on the Condrieu testnet
2. **Local SLED Storage** - Cache contract data locally for faster access and persistence
3. **Price Listener** - Automatically execute transactions when token prices reach thresholds

## Prerequisites

1. **Condrieu Testnet Node**
   - Clone the modified geth: `git clone https://github.com/ethereum/go-ethereum`
   - Switch to the verkle branch: `git checkout verkle`
   - Build it: `make geth`
   - Start the node: `./build/bin/geth --http --ws --verkle`

2. **Rust and Cargo**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **cargo-contract for ink! compilation**
   ```bash
   # Use a specific version to avoid compatibility issues
   cargo install cargo-contract --version 2.0.0
   ```

4. **Substrate Contracts Node**
   
   Instead of using the git installation which causes errors, install from crates.io:
   ```bash
   cargo install contracts-node --locked
   ```

   If you still encounter errors like:
   ```
   error[E0050]: method `dry_run_call` has 2 parameters but the declaration in trait `dry_run_call` has 3
   ```
   
   Use a specific version:
   ```bash
   cargo install contracts-node --version 0.35.0 --locked
   ```

5. **Testnet ETH**
   - Request test ETH from the Condrieu faucet

## Testing the Contract

### Option 1: Automated Testing (Recommended)

The repository includes a test script that automates the entire testing process:

```bash
chmod +x test-on-condrieu.sh
./test-on-condrieu.sh
```

### Option 2: Manual Testing

If you prefer to test manually, follow these steps:

#### 1. Build the Project

```bash
chmod +x build.sh
./build.sh
```

#### 2. Deploy the Contract

```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED deploy
```

Make note of the contract address from the output.

#### 3. Create a Token

```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED create-token --contract YOUR_CONTRACT_ADDRESS --uri "https://example.com/token/1" --supply 1000
```

This creates a new token with the specified URI and initial supply.

#### 4. Verify Storage On-Chain

```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED verify-storage --contract YOUR_CONTRACT_ADDRESS
```

This is similar to what was shown at 3:46 in the video. You'll see:
- Contract metadata
- Storage structure
- Token data with URIs
- Token balances

#### 5. Sync Data to SLED Storage

```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED sync-storage --contract YOUR_CONTRACT_ADDRESS
```

This synchronizes the blockchain data to your local SLED database.

#### 6. Create a Price Listener

```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED create-price-listener --contract YOUR_CONTRACT_ADDRESS --token-id 1 --target-price 150 --action-type sell --amount 100 --price-limit 145
```

This creates a price listener that will sell 100 tokens when the price reaches 150 (as long as it's at least 145).

Other action types:
- **Buy**: `--action-type buy --amount 50 --price-limit 155` (buy 50 tokens when price reaches target, as long as it's below max price)
- **Transfer**: `--action-type transfer --amount 25 --recipient RECIPIENT_ADDRESS` (transfer tokens when price reaches target)

#### 7. Start the Price Listener Service

```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED start-price-listener
```

This starts the price listener service in the background.

For foreground operation with console output:
```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED start-price-listener --foreground true
```

#### 8. Test with a Manual Price Update

```bash
./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed YOUR_SEED update-price --token-id 1 --price 150
```

This updates the price of token ID 1 to 150, which should trigger any price listeners set for that threshold.

## Understanding the Storage System

This implementation uses two complementary storage systems:

### 1. On-Chain Storage (Blockchain)

The ERC1155 contract data is stored on the Condrieu testnet using verkle trees, as demonstrated in the video. This includes:
- Token balances for each account
- Token URIs
- Approval settings

The `storage_validator.rs` module allows you to verify this on-chain storage by computing the verkle tree keys and querying the blockchain state.

### 2. Local Storage (SLED Database)

In addition to on-chain storage, this implementation uses a SLED database for local storage to:
- Cache contract data for faster access
- Store price listeners and trigger conditions
- Enable automatic execution based on price thresholds

The SLED database is stored in the `./erc1155_db` directory by default (configurable with `--storage-path`).

## Price-Triggered Automatic Execution

A key feature of this implementation is the ability to automatically execute transactions when token prices reach specified thresholds:

1. **Create Price Listeners**: Set conditions for automatic execution (sell, buy, transfer)
2. **Start the Listener Service**: A background service monitors price updates
3. **Price Updates**: When a price update matches a listener's threshold, the transaction executes automatically

This system enables the automation of token transactions based on market conditions, as specified in the task.

## Troubleshooting

### Dependency Issues

If you encounter errors related to dependencies or version mismatches:

1. **Use Specific Cargo Versions**:
   ```bash
   # Use nightly Rust for ink! contracts
   rustup install nightly
   rustup target add wasm32-unknown-unknown --toolchain nightly
   
   # Install specific versions of tools
   cargo install cargo-contract --version 2.0.0
   cargo install contracts-node --version 0.35.0 --locked
   ```

2. **Set Environment Variables**:
   ```bash
   # Add this to your shell profile or run before building
   export WASM_BUILD_WORKSPACE_HINT=/path/to/your/project
   export RUST_LOG=debug
   ```

### Runtime Issues

- **Connection Issues**: Make sure your Condrieu node is running and accessible
- **Transaction Failures**: Verify you have enough test ETH
- **Command Not Found**: Ensure the CLI tool was built correctly with `build.sh`
- **Storage Verification Issues**: The contract address might be incorrect or the contract might not be deployed
- **SLED Database Errors**: Check if the database path is writable

If you encounter persistent issues, check the Condrieu testnet documentation or restart your local node.

## Connecting to a Remote Condrieu Node

If you want to connect to a public Condrieu testnet node instead of running your own, modify the node URL in the commands:

```bash
./target/release/erc1155-cli --node-url wss://condrieu-testnet.ethereum.org --seed YOUR_SEED deploy
```
