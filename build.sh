#!/bin/bash
set -e

echo "Building ERC1155 Contract and CLI Tools"
echo "======================================"

# Check for rustup and cargo contract
if ! command -v rustup &> /dev/null; then
    echo "rustup not found. Please install Rust toolchain."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Install nightly toolchain for ink! contracts
echo "Setting up Rust toolchain..."
rustup install nightly
rustup default stable
rustup target add wasm32-unknown-unknown --toolchain nightly

# Install cargo-contract with a specific version
if ! command -v cargo-contract &> /dev/null; then
    echo "cargo-contract not found. Installing version 2.0.0..."
    cargo install cargo-contract --version 2.0.0 --force
else
    echo "cargo-contract found. If you encounter errors, try reinstalling with version 2.0.0"
fi

# Export environment variables to avoid compilation errors
export WASM_BUILD_WORKSPACE_HINT=$(pwd)
echo "Set WASM_BUILD_WORKSPACE_HINT to $(pwd)"

# Build the ERC1155 contract
echo "Building ink! contract..."
cargo +nightly contract build --release

# Check if contract was built
if [ ! -f "target/ink/erc1155.wasm" ]; then
    echo "Contract build failed or output file not found."
    exit 1
fi

# Copy contract to the project root for easy access
cp target/ink/erc1155.wasm .
echo "Contract built and copied to project root: erc1155.wasm"

# Build the CLI tool
echo "Building CLI tool..."
cargo build --release

echo "Build completed successfully!"
echo ""
echo "To deploy the contract, run:"
echo "./target/release/erc1155-cli --node-url ws://127.0.0.1:9944 --seed //Alice deploy"
echo ""
echo "Make sure you have a Substrate node with the contracts pallet running."
echo ""
echo "If you encounter any dependency errors when running, try:"
echo "export WASM_BUILD_WORKSPACE_HINT=$(pwd)" 