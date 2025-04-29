mod storage_validator;
mod error;
mod storage_sled;
mod price_listener;

use sp_core::{sr25519, Pair, H256, crypto::Ss58Codec};
use sp_core::crypto::keccak_256;
use subxt::{
    tx::PairSigner,
    OnlineClient,
    PolkadotConfig,
    config::substrate::{SubstrateExtrinsicParams, SubstrateExtrinsicParamsBuilder},
    ext::scale_encode,
    utils::AccountId32,
};
use ink::env::AccountId;
use std::{str::FromStr, time::Duration};
use codec::{Encode, Decode};
use clap::{Parser, Subcommand};
use hex;
use getrandom;

use storage_sled::{StorageSled, PriceListener, PriceAction, Token, Balance};
use price_listener::PriceListenerService;
use error::CliError;

/// ERC1155 contract deployment and interaction tool
#[derive(Parser)]
#[clap(name = "erc1155-cli")]
struct Cli {
    /// Substrate node URL to connect to
    #[clap(long, default_value = "ws://127.0.0.1:9944")]
    node_url: String,

    /// The seed phrase for the account to use
    #[clap(long, default_value = "//Alice")]
    seed: String,
    
    /// Path to SLED database for local storage
    #[clap(long, default_value = "./erc1155_db")]
    storage_path: String,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a price listener for automatic execution
    CreatePriceListener {
        /// The contract address
        #[clap(long)]
        contract: String,
        
        /// The token ID to monitor
        #[clap(long)]
        token_id: u128,
        
        /// The target price to trigger the action
        #[clap(long)]
        target_price: u128,
        
        /// The action type (sell, buy, transfer)
        #[clap(long)]
        action_type: String,
        
        /// The amount of tokens for the action
        #[clap(long)]
        amount: u128,
        
        /// The price limit (min for sell, max for buy)
        #[clap(long)]
        price_limit: Option<u128>,
        
        /// The recipient address for transfer actions
        #[clap(long)]
        recipient: Option<String>,
    },
    
    /// Start the price listener service
    StartPriceListener {
        /// Run in foreground (true) or background (false)
        #[clap(long, default_value = "false")]
        foreground: bool,
    },
    
    /// Manually update a token price (for testing)
    UpdatePrice {
        /// The token ID to update
        #[clap(long)]
        token_id: u128,
        
        /// The new price
        #[clap(long)]
        price: u128,
    },
    
    /// Sync blockchain data to local SLED storage
    SyncStorage {
        /// The contract address
        #[clap(long)]
        contract: String,
    },
    
    /// Deploy a new ERC1155 contract
    Deploy,
    
    /// Create a new token type in an existing contract
    CreateToken {
        /// The contract address
        #[clap(long)]
        contract: String,
        
        /// The token URI
        #[clap(long)]
        uri: String,
        
        /// Initial supply to mint
        #[clap(long, default_value = "100")]
        supply: u128,
    },
    
    /// Transfer tokens
    Transfer {
        /// The contract address
        #[clap(long)]
        contract: String,
        
        /// The recipient address
        #[clap(long)]
        to: String,
        
        /// The token ID to transfer
        #[clap(long)]
        token_id: u128,
        
        /// Amount to transfer
        #[clap(long)]
        amount: u128,
    },
    
    /// Check token balance
    Balance {
        /// The contract address
        #[clap(long)]
        contract: String,
        
        /// The account to check
        #[clap(long)]
        account: String,
        
        /// The token ID to check
        #[clap(long)]
        token_id: u128,
    },
    
    /// Verify contract storage on-chain
    VerifyStorage {
        /// The contract address
        #[clap(long)]
        contract: String,
        
        /// Optional block number to query
        #[clap(long)]
        block_number: Option<u32>,
    },
}

/// Main application for ERC1155 contract deployment and interaction
#[subxt::subxt(runtime_metadata_path = "metadata.scale")]
pub mod substrate {
    #[subxt::subxt(substitute_type = "frame_support::storage::bounded_vec::BoundedVec<T, S>")]
    use ::std::vec::Vec;
}

// CodeHash type for contracts pallet
type CodeHash<T> = <T as frame_system::Config>::Hash;

// ContractInstantiateResult for contracts pallet
#[derive(Encode, Decode, Debug)]
pub struct ContractInstantiateResult<T> {
    pub result: Result<InstantiateReturnValue<T>, ()>,
}

#[derive(Encode, Decode, Debug)]
pub struct InstantiateReturnValue<T> {
    pub result: Vec<u8>,
    pub account_id: AccountId32,
    pub gas_consumed: u64,
    pub gas_required: u64,
    pub storage_deposit: StorageDeposit<T>,
}

#[derive(Encode, Decode, Debug)]
pub enum StorageDeposit<T> {
    Refund(T),
    Charge(T),
}

// Custom contract call types
#[derive(Debug, Encode, Decode)]
struct BalanceOfParams {
    account: AccountId32,
    id: u128,
}

#[derive(Debug, Encode, Decode)]
struct TransferParams {
    from: AccountId32,
    to: AccountId32, 
    id: u128,
    amount: u128,
    data: Vec<u8>,
}

#[derive(Debug, Encode, Decode)]
struct CreateTokenParams {
    uri: String,
    initial_supply: u128,
}

// Helper function to compute Ethereum-style function selectors
fn compute_selector(signature: &str) -> [u8; 4] {
    let hash = keccak_256(signature.as_bytes());
    let mut selector = [0u8; 4];
    selector.copy_from_slice(&hash[0..4]);
    selector
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Connect to substrate node
    println!("Connecting to substrate node at {}", cli.node_url);
    let api = OnlineClient::<PolkadotConfig>::from_url(cli.node_url.clone()).await?;
    
    // Create a key pair for signing transactions
    let pair = sr25519::Pair::from_string(&cli.seed, None)?;
    let signer = PairSigner::new(pair.clone());
    let account_id = AccountId32::from(signer.account_id().0);
    
    println!("Using account: {}", account_id.to_ss58check());
    
    match cli.command {
        Commands::CreatePriceListener { contract, token_id, target_price, action_type, amount, price_limit, recipient } => {
            let contract_address = AccountId::from_str(&contract)?;
            
            // Initialize the price listener service
            let service = PriceListenerService::new(&cli.storage_path, &cli.node_url)?;
            
            // Create the action based on the type
            let action = match action_type.as_str() {
                "sell" => {
                    let min_price = price_limit.ok_or_else(|| CliError::Other("Min price required for sell action".into()))?;
                    PriceAction::Sell { amount, min_price }
                },
                "buy" => {
                    let max_price = price_limit.ok_or_else(|| CliError::Other("Max price required for buy action".into()))?;
                    PriceAction::Buy { amount, max_price }
                },
                "transfer" => {
                    let to_str = recipient.ok_or_else(|| CliError::Other("Recipient address required for transfer action".into()))?;
                    let to = AccountId::from_str(&to_str)?;
                    PriceAction::Transfer { to, amount }
                },
                _ => return Err(CliError::Other(format!("Unknown action type: {}", action_type)).into()),
            };
            
            // Create the price listener
            service.create_price_listener(token_id, target_price, action)?;
            
            println!("Created price listener for token {} at target price {}", token_id, target_price);
        },
        Commands::StartPriceListener { foreground } => {
            // Initialize the price listener service
            let service = PriceListenerService::new(&cli.storage_path, &cli.node_url)?;
            
            // Start the service
            service.start(&cli.seed)?;
            
            if foreground {
                println!("Price listener service running in foreground. Press Ctrl+C to stop.");
                // Wait indefinitely (until Ctrl+C)
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            } else {
                println!("Price listener service started in background.");
            }
        },
        Commands::UpdatePrice { token_id, price } => {
            // Initialize the price listener service
            let service = PriceListenerService::new(&cli.storage_path, &cli.node_url)?;
            
            // Update the price
            service.update_price(token_id, price)?;
            
            println!("Price updated for token {} to {}", token_id, price);
        },
        Commands::SyncStorage { contract } => {
            let contract_address = AccountId::from_str(&contract)?;
            
            // Initialize SLED storage
            let storage = StorageSled::new(&cli.storage_path)?;
            
            // Store the contract address
            storage.store_contract_address(contract_address.clone())?;
            
            // Sync blockchain data to local storage
            storage.sync_from_blockchain(&api, contract_address).await?;
            
            println!("Blockchain data synced to local storage");
        },
        Commands::Deploy => {
            deploy_contract(&api, &signer).await?;
        },
        Commands::CreateToken { contract, uri, supply } => {
            let contract_address = AccountId::from_str(&contract)?;
            create_token(&api, &signer, contract_address, uri, supply).await?;
            
            // Also store token in local SLED storage
            let storage = StorageSled::new(&cli.storage_path)?;
            
            // Store the contract address
            storage.store_contract_address(contract_address.clone())?;
            
            // Create a token object (with ID 1 as a placeholder - in a real impl this would be retrieved from events)
            let token = Token {
                id: 1, // Placeholder
                uri: uri.clone(),
                creator: AccountId::from(signer.account_id().0),
                total_supply: supply,
                price_threshold: None,
            };
            
            // Store the token
            storage.store_token(token)?;
            
            println!("Token also stored in local SLED database");
        },
        Commands::Transfer { contract, to, token_id, amount } => {
            let contract_address = AccountId::from_str(&contract)?;
            let to_address = AccountId::from_str(&to)?;
            transfer_tokens(&api, &signer, contract_address, to_address, token_id, amount).await?;
            
            // Update balances in local SLED storage
            let storage = StorageSled::new(&cli.storage_path)?;
            
            // Get current balance of sender
            let from_address = AccountId::from(signer.account_id().0);
            let sender_balance = storage.get_balance(&from_address, token_id)?;
            
            // Update sender balance
            let new_sender_balance = Balance {
                account: from_address,
                token_id,
                amount: sender_balance.saturating_sub(amount),
            };
            storage.update_balance(new_sender_balance)?;
            
            // Get current balance of recipient
            let recipient_balance = storage.get_balance(&to_address, token_id)?;
            
            // Update recipient balance
            let new_recipient_balance = Balance {
                account: to_address,
                token_id,
                amount: recipient_balance.saturating_add(amount),
            };
            storage.update_balance(new_recipient_balance)?;
            
            println!("Balances updated in local SLED database");
        },
        Commands::Balance { contract, account, token_id } => {
            let contract_address = AccountId::from_str(&contract)?;
            let account_address = AccountId::from_str(&account)?;
            check_balance(&api, contract_address, account_address, token_id).await?;
        },
        Commands::VerifyStorage { contract, block_number } => {
            let contract_address = AccountId::from_str(&contract)?;
            let block = block_number.unwrap_or(0); // 0 means latest block
            storage_validator::display_contract_state(&api, contract_address, block).await?;
        },
    }
    
    Ok(())
}

/// Deploys a new ERC1155 contract
async fn deploy_contract<T: subxt::Config>(
    api: &OnlineClient<T>,
    signer: &PairSigner<T, sr25519::Pair>,
) -> Result<(), Box<dyn std::error::Error>>
where
    T::AccountId: From<[u8; 32]>,
    <T as subxt::Config>::Address: From<T::AccountId>,
{
    println!("Deploying ERC1155 contract...");
    
    // Load the contract WASM from file
    println!("Loading contract code...");
    let contract_path = "erc1155.wasm";
    let contract_wasm = match std::fs::read(contract_path) {
        Ok(wasm) => wasm,
        Err(e) => {
            println!("Error reading contract file: {}", e);
            println!("Make sure the compiled contract file 'erc1155.wasm' exists in the current directory.");
            return Err(Box::new(e));
        }
    };
    
    // Upload the contract code
    println!("Uploading contract code to blockchain...");
    
    // Create contract upload transaction
    let upload_tx = substrate::tx()
        .contracts()
        .upload_code(
            contract_wasm,
            None, // storage_deposit_limit
            subxt::config::substrate::DefensiveTx::Yes,
        );
    
    // Submit the transaction and wait for it to be included in a block
    let upload_progress = api
        .tx()
        .sign_and_submit_then_watch_default(&upload_tx, signer)
        .await?;
    
    let upload_events = upload_progress.wait_for_finalized_success().await?;
    
    // Extract the code hash from events
    let mut code_hash = H256::default();
    
    for event in upload_events.find_events::<substrate::contracts::events::CodeStored>() {
        if let Ok(ev) = event {
            code_hash = ev.code_hash;
            println!("Contract code uploaded with hash: {:?}", code_hash);
            break;
        }
    }
    
    if code_hash == H256::default() {
        return Err("Failed to extract code hash from events".into());
    }
    
    // Instantiate the contract
    println!("Instantiating contract...");
    
    // No constructor arguments for our ERC1155 contract
    let data = Vec::<u8>::new();
    
    // Salt for address generation (using a random value)
    let mut salt = [0u8; 32];
    getrandom::getrandom(&mut salt)?;
    
    let instantiate_tx = substrate::tx()
        .contracts()
        .instantiate_with_code(
            0u128, // endowment
            10_000_000_000u64, // gas_limit
            None, // storage_deposit_limit
            contract_wasm,
            data, // constructor args
            salt.to_vec(),
        );
    
    // Submit the transaction and wait for it to be included in a block
    let instantiate_progress = api
        .tx()
        .sign_and_submit_then_watch_default(&instantiate_tx, signer)
        .await?;
    
    let instantiate_events = instantiate_progress.wait_for_finalized_success().await?;
    
    // Extract contract address from events
    let mut contract_address = None;
    
    for event in instantiate_events.find_events::<substrate::contracts::events::Instantiated>() {
        if let Ok(ev) = event {
            let deployer: [u8; 32] = ev.deployer.into();
            println!("Contract instantiated by: {:?}", hex::encode(deployer));
            
            let contract: [u8; 32] = ev.contract.into();
            contract_address = Some(AccountId::from(contract));
            println!("Contract deployed at address: 0x{}", hex::encode(contract));
            break;
        }
    }
    
    if let Some(addr) = contract_address {
        println!("Successfully deployed ERC1155 contract");
        println!("Contract Address: 0x{}", hex::encode(addr.as_ref()));
    } else {
        return Err("Failed to extract contract address from events".into());
    }
    
    Ok(())
}

/// Creates a new token type in the ERC1155 contract
async fn create_token<T: subxt::Config>(
    api: &OnlineClient<T>,
    signer: &PairSigner<T, sr25519::Pair>,
    contract_address: AccountId,
    uri: String,
    initial_supply: u128,
) -> Result<(), Box<dyn std::error::Error>>
where
    T::AccountId: From<[u8; 32]>,
    <T as subxt::Config>::Address: From<T::AccountId>,
{
    println!("Creating a new token in contract {}", contract_address);
    
    // Prepare contract call data for create_token
    let params = CreateTokenParams {
        uri,
        initial_supply,
    };
    
    // Selector for create_token function - compute proper selector
    let selector = compute_selector("createToken(string,uint128)");
    
    // Encode the message: selector + params
    let mut message = selector.to_vec();
    message.extend(params.encode());
    
    // Create contract call transaction
    let contract_call_tx = substrate::tx()
        .contracts()
        .call(
            T::AccountId::from(contract_address.0), // Contract address
            0u128,                                  // value to transfer
            10_000_000_000u64,                      // gas limit
            None,                                   // storage deposit limit
            message,                                // encoded message
        );
    
    // Submit transaction
    let tx_progress = api
        .tx()
        .sign_and_submit_then_watch_default(&contract_call_tx, signer)
        .await?;
    
    let tx_events = tx_progress.wait_for_finalized_success().await?;
    
    // Look for TokenCreated event containing the token ID
    let mut token_id = None;
    
    for event in tx_events.find_events::<substrate::contracts::events::ContractEmitted>() {
        if let Ok(ev) = event {
            if ev.contract == T::AccountId::from(contract_address.0) {
                println!("Contract emitted event with data: 0x{}", hex::encode(&ev.data));
                
                // Extract token ID from event data
                // The event format should be TokenCreated(uint128,AccountId,string)
                // First 4 bytes are the event signature, then the token ID (16 bytes)
                if ev.data.len() >= 20 {
                    let event_selector = &ev.data[0..4];
                    // Check if this is the TokenCreated event
                    if event_selector == &compute_selector("TokenCreated(uint128,address,string)")[..] {
                        let mut id_bytes = [0u8; 16];
                        id_bytes.copy_from_slice(&ev.data[4..20]);
                        token_id = Some(u128::from_le_bytes(id_bytes));
                    }
                }
            }
        }
    }
    
    if let Some(id) = token_id {
        println!("Token created successfully with ID: {}", id);
        
        // Verify token on-chain by checking its URI and creator's balance
        println!("\nVerifying token storage on-chain:");
        
        // Check the URI directly from storage
        match storage_validator::verify_token_uri(api, contract_address.clone(), id).await {
            Ok(uri) => println!("Token URI verified: {}", uri),
            Err(e) => println!("Failed to verify token URI: {}", e),
        }
        
        // Check the creator's balance
        let creator_account = AccountId::from(signer.account_id().0);
        match storage_validator::verify_token_balance(api, contract_address, creator_account, id).await {
            Ok(balance) => println!("Creator's balance verified: {}", balance),
            Err(e) => println!("Failed to verify creator's balance: {}", e),
        }
    } else {
        println!("Token created successfully, but couldn't extract token ID from events");
        println!("Check contract storage to verify token creation");
    }
    
    Ok(())
}

/// Transfers tokens between accounts
async fn transfer_tokens<T: subxt::Config>(
    api: &OnlineClient<T>,
    signer: &PairSigner<T, sr25519::Pair>,
    contract_address: AccountId,
    to: AccountId,
    token_id: u128,
    amount: u128,
) -> Result<(), Box<dyn std::error::Error>>
where
    T::AccountId: From<[u8; 32]>,
    <T as subxt::Config>::Address: From<T::AccountId>,
{
    println!("Transferring {} tokens with ID {} to {}", amount, token_id, to);
    
    // Prepare contract call data for safe_transfer_from
    let params = TransferParams {
        from: AccountId32::from(signer.account_id().0),
        to: AccountId32::from(to.0),
        id: token_id,
        amount,
        data: Vec::new(),
    };
    
    // Selector for safe_transfer_from function - compute proper selector
    let selector = compute_selector("safeTransferFrom(address,address,uint128,uint128,bytes)");
    
    // Encode the message: selector + params
    let mut message = selector.to_vec();
    message.extend(params.encode());
    
    // Create contract call transaction
    let contract_call_tx = substrate::tx()
        .contracts()
        .call(
            T::AccountId::from(contract_address.0), // Contract address
            0u128,                                  // value to transfer
            10_000_000_000u64,                      // gas limit
            None,                                   // storage deposit limit
            message,                                // encoded message
        );
    
    // Submit transaction
    let tx_progress = api
        .tx()
        .sign_and_submit_then_watch_default(&contract_call_tx, signer)
        .await?;
    
    let tx_events = tx_progress.wait_for_finalized_success().await?;
    
    println!("Transfer completed successfully");
    
    // Check if there were any contract events emitted
    for event in tx_events.find_events::<substrate::contracts::events::ContractEmitted>() {
        if let Ok(ev) = event {
            if ev.contract == T::AccountId::from(contract_address.0) {
                println!("Contract emitted event with data: 0x{}", hex::encode(&ev.data));
            }
        }
    }
    
    Ok(())
}

/// Checks the balance of an account for a specific token
async fn check_balance<T: subxt::Config>(
    api: &OnlineClient<T>,
    contract_address: AccountId,
    account: AccountId,
    token_id: u128,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking balance of account {} for token ID {}", account, token_id);
    
    // First try to read directly from storage
    let balance = storage_validator::verify_token_balance(
        api, 
        contract_address.clone(), 
        account.clone(), 
        token_id
    ).await?;
    
    println!("Token balance from storage: {}", balance);
    
    // In addition, query using a contract call
    // Prepare contract call data for balance_of
    let params = BalanceOfParams {
        account: AccountId32::from(account.0),
        id: token_id,
    };
    
    // Selector for balance_of function
    let selector = [0x00, 0x01, 0x02, 0x03]; 
    
    // Encode the message: selector + params
    let mut message = selector.to_vec();
    message.extend(params.encode());
    
    // Create contract call for read-only query
    let result = api.rpc().state_call(
        "ContractsApi_call",
        scale::Encode::encode(&(
            T::AccountId::from(contract_address.0), // Contract address
            0u128,                                  // value to transfer
            10_000_000_000u64,                      // gas limit
            None::<()>,                             // storage deposit limit
            message,                                // encoded message
        )).as_slice(),
    ).await?;
    
    if !result.is_empty() {
        // Decode the result
        if result.len() >= 16 {
            let mut balance_bytes = [0u8; 16];
            balance_bytes.copy_from_slice(&result[0..16]);
            let contract_balance = u128::from_le_bytes(balance_bytes);
            println!("Balance from contract call: {}", contract_balance);
        } else {
            println!("Couldn't decode balance from contract call");
        }
    } else {
        println!("No result returned from contract call");
    }
    
    Ok(())
} 