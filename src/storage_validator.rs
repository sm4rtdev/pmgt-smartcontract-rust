use subxt::{
    OnlineClient,
    blocks::Block,
    storage::Storage,
    error::Error,
    utils::{AccountId32, H256},
    PolkadotConfig
};
use hex;
use ink::env::AccountId;
use sp_core::H160;
use sp_core::{sr25519, crypto::Ss58Codec};
use codec::{Encode, Decode};
use std::collections::HashMap;

/// This module provides utilities to verify contract storage on the blockchain.

/// Fetches and displays contract state
pub async fn display_contract_state<T: subxt::Config>(
    api: &OnlineClient<T>,
    contract_address: AccountId,
    block_number: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Contract State Verification ===");
    println!("Contract Address: {}", contract_address);
    
    // Convert ink AccountId to substrate AccountId32
    let contract_account = AccountId32::from(contract_address.0);
    
    // Get the contract code hash
    let code_hash_key = compute_contract_code_hash_key(contract_address);
    let code_hash_key_hex = format!("0x{}", hex::encode(code_hash_key.as_bytes()));
    
    let code_hash_value = api.rpc().storage(
        &code_hash_key_hex,
        None
    ).await?;
    
    if let Some(value) = code_hash_value {
        println!("Contract Code Hash: 0x{}", hex::encode(&value.0));
    } else {
        println!("Contract code hash not found - contract may not exist");
        return Ok(());
    }
    
    println!("\n--- Storage Structure ---");
    println!("The ERC1155 contract has the following storage layout:");
    println!("1. Balances: Mapping((TokenId, AccountId) => Balance)");
    println!("2. Approvals: Mapping((Owner, Operator) => bool)");
    println!("3. Token URIs: Mapping(TokenId => String)");
    println!("4. Token ID Nonce: u128");
    
    // Fetch the token ID nonce to know how many tokens exist
    let nonce_key = compute_nonce_storage_key(contract_address);
    let nonce_key_hex = format!("0x{}", hex::encode(nonce_key.as_bytes()));
    
    let nonce_value = api.rpc().storage(
        &nonce_key_hex,
        None
    ).await?;
    
    let mut token_count = 0;
    
    if let Some(value) = nonce_value {
        if !value.0.is_empty() {
            // Decode the nonce value
            token_count = match u128::decode(&mut &value.0[..]) {
                Ok(n) => n,
                Err(_) => {
                    if value.0.len() >= 16 {
                        let mut nonce_bytes = [0u8; 16];
                        nonce_bytes.copy_from_slice(&value.0[0..16]);
                        u128::from_le_bytes(nonce_bytes)
                    } else {
                        0
                    }
                }
            };
            println!("\nToken ID Nonce: {} (indicates {} tokens have been created)", 
                    token_count, token_count);
        }
    } else {
        println!("\nToken ID Nonce not found in storage");
    }
    
    println!("\n--- Token Data ---");
    // Query all tokens up to the nonce value
    let mut token_data = HashMap::new();
    
    for token_id in 1..=token_count {
        // Get URI for each token
        match verify_token_uri(api, contract_address.clone(), token_id).await {
            Ok(uri) => {
                if !uri.is_empty() {
                    token_data.insert(token_id, uri);
                }
            },
            Err(_) => {}
        }
    }
    
    for (id, uri) in token_data {
        println!("Token #{}: URI = {}", id, uri);
    }
    
    println!("\n--- Balance Data ---");
    println!("Scanning for non-zero balances in the tokens...");
    
    // Known test accounts to check (user can extend this based on their usage)
    let test_accounts = vec![
        contract_address.clone(), // Contract itself
        AccountId::from([0; 32]), // Zero address
        // Additional accounts could be added from command line or configuration
    ];
    
    let mut found_balances = false;
    
    // Check balances for each token and known account
    for token_id in 1..=token_count {
        for account in &test_accounts {
            match verify_token_balance(api, contract_address.clone(), account.clone(), token_id).await {
                Ok(balance) => {
                    if balance > 0 {
                        found_balances = true;
                        println!("Account {} has {} of token #{}", 
                                account, balance, token_id);
                    }
                },
                Err(e) => {
                    println!("Error checking balance: {}", e);
                }
            }
        }
    }
    
    if !found_balances {
        println!("No non-zero balances found in test accounts. This doesn't mean there are no balances at all.");
        println!("To check specific accounts, use the 'balance' command.");
    }
    
    println!("\n=== End of Contract State ===");
    
    Ok(())
}

/// Attempts to verify a token balance directly from contract storage
pub async fn verify_token_balance<T: subxt::Config>(
    api: &OnlineClient<T>,
    contract_address: AccountId,
    account: AccountId,
    token_id: u128,
) -> Result<u128, Box<dyn std::error::Error>> {
    // Convert ink AccountId to substrate AccountId32
    let contract_account = AccountId32::from(contract_address.0);
    let user_account = AccountId32::from(account.0);
    
    println!("Verifying balance from storage for account {} and token {}", 
             hex::encode(user_account.as_ref()),
             token_id);
    
    // Compute the storage key for the balances mapping
    let storage_key = compute_balance_storage_key(contract_address, account, token_id);
    
    // Convert to hex string for API call
    let storage_key_hex = format!("0x{}", hex::encode(storage_key.as_bytes()));
    
    // Try to get the storage value
    let storage_value = api.rpc().storage(
        &storage_key_hex,
        None
    ).await?;
    
    if let Some(value) = storage_value {
        // Decode the balance
        if !value.0.is_empty() {
            // For ink! contracts, the storage value is typically SCALE encoded
            let balance = match u128::decode(&mut &value.0[..]) {
                Ok(b) => b,
                Err(_) => {
                    // If direct decoding fails, try parsing as bytes
                    if value.0.len() >= 16 {
                        let mut balance_bytes = [0u8; 16];
                        balance_bytes.copy_from_slice(&value.0[0..16]);
                        u128::from_le_bytes(balance_bytes)
                    } else {
                        0
                    }
                }
            };
            
            println!("Successfully decoded balance: {}", balance);
            return Ok(balance);
        }
    }
    
    // If not found or couldn't decode, return 0
    println!("Could not find balance in storage, returning 0");
    Ok(0)
}

/// Attempts to identify what a storage item might be based on its key and value
fn identify_storage_item(key: &str, value: &[u8]) {
    // This is a simplified implementation
    // In a real-world scenario, you'd need much more sophisticated parsing based on the contract's storage layout
    
    if key.contains("balance") || key.contains("Balance") {
        println!("This appears to be a balance entry");
        
        if value.len() >= 16 {
            let mut balance_bytes = [0u8; 16];
            balance_bytes.copy_from_slice(&value[0..16]);
            let balance = u128::from_le_bytes(balance_bytes);
            println!("  Decoded balance: {}", balance);
        }
    } else if key.contains("owner") || key.contains("Owner") {
        println!("This appears to be an ownership entry");
        
        if value.len() >= 32 {
            println!("  Owner might be: 0x{}", hex::encode(&value[0..32]));
        }
    } else if key.contains("uri") || key.contains("URI") {
        println!("This appears to be a URI entry");
        
        // Try to decode as string
        match String::from_utf8(value.to_vec()) {
            Ok(s) => println!("  Decoded URI: {}", s),
            Err(_) => println!("  Could not decode URI as UTF-8 string"),
        }
    } else {
        println!("Unknown storage item type");
    }
}

/// Function to retrieve and display token URI directly from blockchain storage
pub async fn verify_token_uri(
    api: &OnlineClient<PolkadotConfig>,
    contract_address: AccountId,
    token_id: u128,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Verifying token URI directly from blockchain storage...");
    
    // Compute the storage key for the URI mapping
    let storage_key = compute_uri_storage_key(contract_address, token_id);
    
    // Convert to hex string for API call
    let storage_key_hex = format!("0x{}", hex::encode(storage_key.as_bytes()));
    
    // Try to get the storage value
    let storage_value = api.rpc().storage(
        &storage_key_hex,
        None
    ).await?;
    
    if let Some(value) = storage_value {
        // Decode the URI
        if !value.0.is_empty() {
            // Try to decode as a SCALE-encoded string
            match String::decode(&mut &value.0[..]) {
                Ok(uri) => {
                    println!("URI found in blockchain storage: {}", uri);
                    return Ok(uri);
                },
                Err(_) => {
                    // If SCALE decoding fails, try UTF-8 decoding
                    match String::from_utf8(value.0.clone()) {
                        Ok(uri) => {
                            println!("URI found (raw UTF-8) in storage: {}", uri);
                            return Ok(uri);
                        },
                        Err(_) => {
                            println!("Found data but could not decode as string: 0x{}", 
                                    hex::encode(&value.0));
                            return Ok(format!("0x{}", hex::encode(&value.0)));
                        }
                    }
                }
            }
        }
    }
    
    println!("No URI found in storage");
    Ok(String::new())
}

// Actual storage key computation functions that match ink! contract storage layout

fn compute_balance_storage_key(
    contract_address: AccountId,
    account: AccountId,
    token_id: u128,
) -> H256 {
    // Storage layout for ink! contracts:
    // 1. Contract namespace: blake2_128_concat(contract_address)
    // 2. Field identifier: twox_128("balances")
    // 3. Map key: blake2_128_concat((token_id, account))
    
    // Step 1: Create the map key
    let mut token_id_bytes = token_id.encode();
    let mut account_bytes = account.encode();
    let mut map_key = Vec::new();
    map_key.append(&mut token_id_bytes);
    map_key.append(&mut account_bytes);
    
    // Compose the full storage key
    let contract_prefix = blake2_128_concat(contract_address.encode().as_slice());
    let field_identifier = twox_128(b"balances");
    let encoded_map_key = blake2_128_concat(&map_key);
    
    // Combine all parts
    let mut full_key = Vec::new();
    full_key.extend_from_slice(&contract_prefix);
    full_key.extend_from_slice(&field_identifier);
    full_key.extend_from_slice(&encoded_map_key);
    
    // Convert to H256 (padded if needed)
    let mut result = [0u8; 32];
    let len = std::cmp::min(full_key.len(), 32);
    result[..len].copy_from_slice(&full_key[..len]);
    
    H256::from(result)
}

fn compute_uri_storage_key(
    contract_address: AccountId,
    token_id: u128,
) -> H256 {
    // Storage layout for token URIs, similar to balances but with different field name
    
    // Contract namespace
    let contract_prefix = blake2_128_concat(contract_address.encode().as_slice());
    // Field identifier
    let field_identifier = twox_128(b"token_uris");
    // Map key
    let encoded_map_key = blake2_128_concat(&token_id.encode());
    
    // Combine all parts
    let mut full_key = Vec::new();
    full_key.extend_from_slice(&contract_prefix);
    full_key.extend_from_slice(&field_identifier);
    full_key.extend_from_slice(&encoded_map_key);
    
    // Convert to H256
    let mut result = [0u8; 32];
    let len = std::cmp::min(full_key.len(), 32);
    result[..len].copy_from_slice(&full_key[..len]);
    
    H256::from(result)
}

fn compute_nonce_storage_key(contract_address: AccountId) -> H256 {
    // Contract namespace
    let contract_prefix = blake2_128_concat(contract_address.encode().as_slice());
    // Field identifier for token_id_nonce
    let field_identifier = twox_128(b"token_id_nonce");
    
    // Combine parts
    let mut full_key = Vec::new();
    full_key.extend_from_slice(&contract_prefix);
    full_key.extend_from_slice(&field_identifier);
    
    // Convert to H256
    let mut result = [0u8; 32];
    let len = std::cmp::min(full_key.len(), 32);
    result[..len].copy_from_slice(&full_key[..len]);
    
    H256::from(result)
}

fn compute_contract_code_hash_key(contract_address: AccountId) -> H256 {
    // For Substrate contracts pallet
    let account_id = AccountId32::from(contract_address.0);
    let storage_prefix = twox_128(b"Contracts");
    let storage_item_prefix = twox_128(b"ContractInfoOf");
    
    // Combine with account
    let mut full_key = Vec::new();
    full_key.extend_from_slice(&storage_prefix);
    full_key.extend_from_slice(&storage_item_prefix);
    full_key.extend_from_slice(&blake2_128_concat(account_id.as_ref()));
    
    // Convert to H256
    let mut result = [0u8; 32];
    let len = std::cmp::min(full_key.len(), 32);
    result[..len].copy_from_slice(&full_key[..len]);
    
    H256::from(result)
}

// Utility functions for storage key calculation
fn blake2_128_concat(data: &[u8]) -> Vec<u8> {
    let hash = sp_core::blake2_128(data);
    let mut result = hash.to_vec();
    result.extend_from_slice(data);
    result
}

fn twox_128(data: &[u8]) -> [u8; 16] {
    let hash = sp_core::twox_128(data);
    hash
} 