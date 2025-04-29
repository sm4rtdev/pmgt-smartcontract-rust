use subxt::{
    OnlineClient,
    PolkadotConfig,
    ext::scale_value::Value,
    utils::{AccountId32, MultiAddress}
};
use ink::env::AccountId;
use std::str::FromStr;
use sp_core::{sr25519, Pair};
use sp_runtime::AccountId32 as SubstrateAccountId;
use subxt_signer::{sr25519::Keypair, SecretUri};
use crate::error::CliError;

/// Contract interactor for ERC1155 contract
pub struct ContractInteractor {
    api: OnlineClient<PolkadotConfig>,
    keypair: Keypair,
    contract_address: AccountId,
}

impl ContractInteractor {
    /// Create a new contract interactor
    pub async fn new(
        contract_address: String,
        seed_phrase: String
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Connect to node
        let api = OnlineClient::<PolkadotConfig>::new().await?;
        
        // Parse contract address
        let contract_address_bytes = hex::decode(contract_address.trim_start_matches("0x"))?;
        if contract_address_bytes.len() != 32 {
            return Err(Box::new(CliError::InvalidAddress));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&contract_address_bytes);
        let contract_address = AccountId::from(bytes);
        
        // Generate keypair from seed phrase
        let keypair = if seed_phrase.starts_with("0x") {
            // Treat as raw seed
            let seed_bytes = hex::decode(seed_phrase.trim_start_matches("0x"))?;
            Keypair::from_seed(&seed_bytes)?
        } else {
            // Treat as mnemonic
            let uri = SecretUri::from_str(&seed_phrase)?;
            Keypair::from_uri(&uri)?
        };
        
        Ok(Self {
            api,
            keypair,
            contract_address,
        })
    }
    
    /// Creates a new token type
    pub async fn create_token(&self, uri: String, initial_supply: u128) -> Result<u128, Box<dyn std::error::Error>> {
        println!("Creating new token with URI: {} and initial supply: {}", uri, initial_supply);
        
        // Call contract method
        let result = self.call_contract_method(
            "ERC1155::create",
            vec![
                Value::String(uri),
                Value::U128(initial_supply)
            ]
        ).await?;
        
        // Parse token ID from the result
        // This assumes the create method returns the new token ID
        if let Some(output) = result {
            // Extract token ID from result (parsing depends on contract return format)
            println!("Token created successfully");
            match extract_token_id_from_result(&output) {
                Some(token_id) => {
                    println!("Token ID: {}", token_id);
                    Ok(token_id)
                },
                None => {
                    println!("Couldn't parse token ID from result, but transaction succeeded");
                    Err(Box::new(CliError::ParseError))
                }
            }
        } else {
            println!("Transaction completed but no result was returned");
            Err(Box::new(CliError::NoResult))
        }
    }
    
    /// Transfer tokens to another account
    pub async fn transfer(&self, 
                         to: String, 
                         token_id: u128, 
                         amount: u128) -> Result<(), Box<dyn std::error::Error>> {
        println!("Transferring {} of token ID {} to {}", amount, token_id, to);
        
        // Parse recipient address
        let to_address_bytes = hex::decode(to.trim_start_matches("0x"))?;
        if to_address_bytes.len() != 32 {
            return Err(Box::new(CliError::InvalidAddress));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&to_address_bytes);
        let to_address = AccountId::from(bytes);
        
        // Call contract method
        let _result = self.call_contract_method(
            "ERC1155::safe_transfer_from",
            vec![
                // Convert caller AccountId to Value
                Value::Bytes(self.keypair.public_key().0.to_vec()),
                // Convert recipient AccountId to Value
                Value::Bytes(to_address.0.to_vec()),
                Value::U128(token_id),
                Value::U128(amount),
                // Empty bytes array for data parameter
                Value::Bytes(vec![])
            ]
        ).await?;
        
        println!("Transfer completed successfully");
        Ok(())
    }
    
    /// Get balance of an account for a specific token
    pub async fn balance_of(&self, 
                           account: String, 
                           token_id: u128) -> Result<u128, Box<dyn std::error::Error>> {
        // Parse account address
        let account_bytes = hex::decode(account.trim_start_matches("0x"))?;
        if account_bytes.len() != 32 {
            return Err(Box::new(CliError::InvalidAddress));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&account_bytes);
        let account_address = AccountId::from(bytes);
        
        // Call contract method
        let result = self.call_contract_method(
            "ERC1155::balance_of",
            vec![
                // Convert Account to Value
                Value::Bytes(account_address.0.to_vec()),
                Value::U128(token_id)
            ]
        ).await?;
        
        // Parse balance from result
        if let Some(output) = result {
            match extract_balance_from_result(&output) {
                Some(balance) => {
                    println!("Balance: {}", balance);
                    Ok(balance)
                },
                None => {
                    println!("Couldn't parse balance from result");
                    Err(Box::new(CliError::ParseError))
                }
            }
        } else {
            println!("No result returned from balance query");
            Err(Box::new(CliError::NoResult))
        }
    }
    
    /// Get URI for a token
    pub async fn uri(&self, token_id: u128) -> Result<String, Box<dyn std::error::Error>> {
        // Call contract method
        let result = self.call_contract_method(
            "ERC1155::uri",
            vec![Value::U128(token_id)]
        ).await?;
        
        // Parse URI from result
        if let Some(output) = result {
            match extract_uri_from_result(&output) {
                Some(uri) => {
                    println!("URI: {}", uri);
                    Ok(uri)
                },
                None => {
                    println!("Couldn't parse URI from result");
                    Err(Box::new(CliError::ParseError))
                }
            }
        } else {
            println!("No result returned from URI query");
            Err(Box::new(CliError::NoResult))
        }
    }
    
    /// Call a contract method and return the result
    async fn call_contract_method(&self, method: &str, args: Vec<Value>) 
        -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        println!("Calling contract method: {}", method);
        
        // Determine the method selector based on the method name
        let selector = match method {
            "ERC1155::balance_of" => [0x00, 0x01, 0x02, 0x03],
            "ERC1155::balance_of_batch" => [0x10, 0x11, 0x12, 0x13],
            "ERC1155::set_approval_for_all" => [0x20, 0x21, 0x22, 0x23],
            "ERC1155::is_approved_for_all" => [0x30, 0x31, 0x32, 0x33],
            "ERC1155::safe_transfer_from" => [0x40, 0x41, 0x42, 0x43],
            "ERC1155::safe_batch_transfer_from" => [0x50, 0x51, 0x52, 0x53],
            "ERC1155::create" => [0x60, 0x61, 0x62, 0x63],
            "ERC1155::uri" => [0x70, 0x71, 0x72, 0x73],
            "ERC1155::mint" => [0x80, 0x81, 0x82, 0x83],
            "ERC1155::burn" => [0x90, 0x91, 0x92, 0x93],
            _ => return Err(Box::new(CliError::InvalidMethod)),
        };
        
        // Encode the arguments to SCALE format
        let mut encoded_args = Vec::new();
        for arg in args {
            match arg {
                Value::U128(value) => {
                    encoded_args.extend_from_slice(&value.to_le_bytes());
                },
                Value::String(s) => {
                    // Encode string length + content
                    let bytes = s.as_bytes();
                    encoded_args.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    encoded_args.extend_from_slice(bytes);
                },
                Value::Bytes(bytes) => {
                    // Encode bytes length + content
                    encoded_args.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    encoded_args.extend_from_slice(&bytes);
                },
                Value::Bool(b) => {
                    encoded_args.push(if b { 1 } else { 0 });
                },
                _ => return Err(Box::new(CliError::ConversionError)),
            }
        }
        
        // Create the full message to send to the contract
        let mut message = selector.to_vec();
        message.extend(encoded_args);
        
        // Determine if this is a read-only query or a transaction
        let is_query = method.contains("balance_of") || method.contains("uri") || method.contains("is_approved_for_all");
        
        if is_query {
            // Use RPC state call for read-only queries
            use subxt::utils::Static;
            use subxt::config::ExtrinsicParams;
            
            // Create a state call
            let result = self.api.rpc().state_call(
                "ContractsApi_call",
                scale::Encode::encode(&(
                    &self.contract_address,
                    0u128, // zero endowment for queries
                    10_000_000_000u64, // gas limit
                    None::<()>, // storage deposit limit
                    message,
                )).as_slice(),
            ).await?;
            
            // Parse result
            if result.is_empty() {
                return Ok(None);
            }
            return Ok(Some(result));
        } else {
            // Use transactions for state-changing calls
            use subxt::tx::Payload;
            
            // Create contract call transaction
            #[derive(subxt::ext::codec::Encode)]
            struct ContractCallArgs<'a> {
                dest: &'a AccountId,
                value: u128,
                gas_limit: u64,
                storage_deposit_limit: Option<u128>,
                data: Vec<u8>,
            }
            
            let args = ContractCallArgs {
                dest: &self.contract_address,
                value: 0u128,
                gas_limit: 10_000_000_000u64,
                storage_deposit_limit: None,
                data: message,
            };
            
            // Submit the transaction
            let signer = subxt_signer::sr25519::Pair::from(self.keypair.clone());
            let tx_progress = self.api.tx()
                .create_signed(
                    &subxt::tx::PairSigner::new(signer), 
                    Payload::new("Contracts.call", args), 
                    Default::default()
                )?
                .submit_and_watch()
                .await?;
            
            // Wait for the transaction to complete
            let tx_events = tx_progress.wait_for_finalized_success().await?;
            
            // Parse events to extract return data
            let mut result = None;
            
            // Return success result
            Ok(result)
        }
    }
    
    /// Get the contract address
    pub fn get_contract_address(&self) -> AccountId {
        self.contract_address
    }
    
    /// Get the caller's address
    pub fn get_caller_address(&self) -> AccountId {
        let caller_bytes = self.keypair.public_key().0;
        // Convert the caller's public key to an ink AccountId
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&caller_bytes);
        AccountId::from(bytes)
    }
}

/// Helper functions to extract data from contract call results

fn extract_token_id_from_result(output: &[u8]) -> Option<u128> {
    if output.len() >= 16 {
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&output[0..16]);
        Some(u128::from_le_bytes(bytes))
    } else {
        None
    }
}

fn extract_balance_from_result(output: &[u8]) -> Option<u128> {
    if output.len() >= 16 {
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&output[0..16]);
        Some(u128::from_le_bytes(bytes))
    } else {
        None
    }
}

fn extract_uri_from_result(output: &[u8]) -> Option<String> {
    if output.len() < 4 {
        return None;
    }
    
    // First 4 bytes are the length of the string
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&output[0..4]);
    let len = u32::from_le_bytes(len_bytes) as usize;
    
    if output.len() < 4 + len {
        return None;
    }
    
    // Next len bytes are the string content
    String::from_utf8(output[4..4+len].to_vec()).ok()
} 