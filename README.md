# ERC1155 NFT Smart Contract

This repository contains an implementation of the ERC1155 multi-token standard in ink!, designed for deployment on the Aleph Zero blockchain. The contract includes functionality for creating, minting, and transferring both NFTs and fungible tokens, plus a specialized airdrop feature for NFT holders.

## Project Structure

```
/
├── erc1155/                      # Main contract directory
│   ├── lib.rs                    # Contract implementation
│   ├── Cargo.toml                # Contract dependencies
│   └── Cargo.lock                # Locked dependencies
├── erc1155.contract              # Compiled, deployable contract file
├── src/                          # Utility and helper code
│   ├── lib.rs                    # Library code
│   ├── main.rs                   # CLI implementation
│   ├── contract_interactor.rs    # Contract interaction utilities
│   ├── price_listener.rs         # Price monitoring utilities
│   ├── storage_sled.rs           # Storage implementation
│   ├── storage_validator.rs      # Storage validation logic
│   └── error.rs                  # Error handling
└── .gitignore                    # Git configuration
```

## Features

- **ERC1155 Standard Compliance**: Create both NFTs and fungible tokens in one contract
- **Token Creation**: Create new token types with metadata 
- **Minting**: Mint tokens to any address
- **Batch Operations**: Transfer multiple tokens in a single transaction
- **Custom Airdrop**: Distribute fungible tokens to NFT holders with `airdrop_to_nft_holders`
- **Role-Based Access Control**: Owner/admin privileges for key operations

## Development Requirements

- Rust and Cargo
- The `cargo-contract` CLI tool
- ink! - The Substrate smart contract language
- A local Substrate node or Aleph Zero testnet for deployment

## Steps to Create and Deploy the Contract

### 1. Set Up the Development Environment

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install the nightly toolchain and WebAssembly target
rustup install nightly
rustup default stable
rustup target add wasm32-unknown-unknown --toolchain nightly

# Install cargo-contract (ink! contract tool)
cargo install cargo-contract --force --version 2.0.0
```

### 2. Create a New ink! Project

```bash
# Create a new contract
cargo contract new erc1155

# Navigate to the project
cd erc1155
```

### 3. Implement the ERC1155 Contract

Modify the `lib.rs` file to implement:
- Token creation and management
- Balance tracking
- Transfer functionality 
- Airdrop feature for NFT holders
- Events for transfers and approvals

### 4. Build the Contract

```bash
# Build the contract
cargo +nightly contract build --release
```

The build process generates:
- `erc1155.wasm`: WebAssembly bytecode
- `erc1155.json`: Contract metadata
- `erc1155.contract`: Combined deployable file

### 5. Deploy on Aleph Zero Testnet

1. Go to [Substrate Contracts UI](https://contracts-ui.substrate.io/)
2. Connect to Aleph Zero Testnet:
   - Select custom endpoint: `wss://condrieu-public.aleph-zero-testnet.io`
3. Connect your Polkadot{.js} wallet with testnet AZERO
4. Upload & deploy the contract:
   - Upload the `.contract` file
   - Set a name for your contract
   - Use default constructor without parameters
   - Set appropriate gas limits
   - Deploy

### 6. Interact with Your Contract

After deployment, you can:
1. Create tokens with `create_token`
2. Mint NFTs to addresses with `mint`
3. Transfer tokens with `transfer` or `transfer_batch` 
4. Airdrop fungible tokens to NFT holders with `airdrop_to_nft_holders`

## CLI Tools (Optional)

The `src` directory contains CLI utilities for interacting with the contract programmatically:
- Token price monitoring
- Batch operations
- Storage utilities

## License

[MIT License](LICENSE) 